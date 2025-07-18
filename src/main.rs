use crate::error::APIError;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    serve, Json, Router,
};
use sea_orm::{
    sqlx::types::chrono::{self, Utc},
    ActiveModelTrait, Database, DatabaseConnection, EntityTrait,
};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    fs::Permissions, io, os::unix::fs::PermissionsExt, path::PathBuf, sync::Arc, time::SystemTime,
};
use tokio::{
    fs::{self},
    net::TcpListener,
};
use tracing::Level;
use tracing_subscriber::fmt::Subscriber;

mod entity;
mod error;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let app_url = "localhost:3000";
    let subscriber = Subscriber::builder()
        .with_writer(io::stderr)
        .with_max_level(
            dotenv::var("MAX_LOG_LEVEL")
                .map(|level| level.parse().unwrap())
                .unwrap_or(Level::WARN),
        )
        .with_file(true)
        .pretty()
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let app = Router::new()
        .route("/health", get(health_check_handler))
        .route("/kiosk-version", post(create_kiosk_version))
        .route("/latest-version", get(get_latest_version))
        .route(
            "/download/{version}/{platform}/{filename}",
            get(download_file),
        );
    let listener = TcpListener::bind(app_url).await.unwrap();
    serve(listener, app).await.unwrap();
}

pub async fn health_check_handler() -> impl IntoResponse {
    "OK"
}

#[derive(Serialize, Deserialize)]
pub struct CreateKioskVersionRequest {
    pub version: String,
    pub notes: String,
}

// TODO
// - [x] create versioning enpoint
// - [x] create folder base version name
// - [x] validate folder if exist
// - [x] checking last created folder
// - [x] checking isi folder
// - [x] notes input ke txt

pub async fn create_kiosk_version(
    request: Json<CreateKioskVersionRequest>,
) -> Result<StatusCode, APIError> {
    let kiosk_directory = dotenv::var("KIOSK_DIRECTORY").unwrap();
    let folder_version_name = request.version.clone();
    let kiosk_version_directory =
        kiosk_directory.clone() + &String::from("/") + &folder_version_name;

    let platforms: Vec<String> = vec![
        "windows_x86_64".to_string(),
        "linux_x86_64".to_string(),
        "darwin_x86_64".to_string(),
        "darwin_aarch64".to_string(),
    ];

    // find folder if exist
    match fs::try_exists(kiosk_version_directory.clone()).await {
        Ok(exists) => {
            if exists {
                tracing::error!(
                    "failed to create folder {} because folder already exists",
                    folder_version_name
                );
                return Err(APIError::FolderExist);
            } else {
                fs::create_dir(kiosk_version_directory.clone())
                    .await
                    .inspect_err(|e| {
                        tracing::error!("failed to create kiosk directory: {:?}", e)
                    })?;

                let permissions = Permissions::from_mode(0o755);

                // set permission
                fs::set_permissions(kiosk_version_directory.clone(), permissions)
                    .await
                    .inspect_err(|e| tracing::error!("failed to set permission: {}", e))?;

                // writes note into txt file
                let content = request.notes.clone();
                fs::write(
                    kiosk_version_directory.clone() + &String::from("/") + "notes.txt",
                    content,
                )
                .await
                .inspect_err(|e| {
                    tracing::error!("failed to write file: {}", e);
                })?;

                for platform in platforms {
                    let kiosk_version_platform_directory =
                        kiosk_version_directory.clone() + &String::from("/") + &platform;
                    fs::create_dir(kiosk_version_platform_directory.clone())
                        .await
                        .inspect_err(|e| {
                            tracing::error!("failed to create kiosk directory: {:?}", e)
                        })?;
                }
            }
        }
        Err(e) => {
            tracing::error!("failed to check if folder exists: {}", e);
            return Err(APIError::Internal);
        }
    }

    Ok(StatusCode::OK)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformDetails {
    pub signature: String,
    pub url: String,
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Platforms {
    #[serde(rename = "linux-x86_64")]
    pub linux_x86_64: PlatformDetails,
    #[serde(rename = "windows-x86_64")]
    pub windows_x86_64: PlatformDetails,
    #[serde(rename = "darwin-x86_64")]
    pub darwin_x86_64: PlatformDetails,
    #[serde(rename = "darwin-aarch64")]
    pub darwin_aarch64: PlatformDetails,
}

impl Platforms {
    fn iter(&self) -> impl Iterator<Item = (&str, &PlatformDetails)> {
        vec![
            ("linux_x86_64", &self.linux_x86_64),
            ("windows_x86_64", &self.windows_x86_64),
            ("darwin_x86_64", &self.darwin_x86_64),
            ("darwin_aarch64", &self.darwin_aarch64),
        ]
        .into_iter()
    }
}

impl Platforms {
    // Returns a mutable iterator over all PlatformDetails
    fn iter_mut(&mut self) -> impl Iterator<Item = &mut PlatformDetails> {
        vec![
            &mut self.linux_x86_64,
            &mut self.windows_x86_64,
            &mut self.darwin_x86_64,
            &mut self.darwin_aarch64,
        ]
        .into_iter()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KioskVersionResponse {
    pub version: String,
    pub notes: String,
    #[serde(rename = "pub_date")]
    pub pub_date: String,
    pub platforms: Platforms,
}

// TODO
// - [x] get latest version folder
// - [x] get latest version folder name
// - [x] check isi folder terbaru
// - [x] jika isi folder terbaru kosong maka return folder terbaru yang ada isinya

pub async fn get_latest_version() -> Result<Json<KioskVersionResponse>, APIError> {
    let kiosk_directory = dotenv::var("KIOSK_DIRECTORY").unwrap();
    let mut modified_date: SystemTime = SystemTime::UNIX_EPOCH;
    let kiosk_url = dotenv::var("KIOSK_DOWNLOADABLE_URL").unwrap();

    let mut platforms = Platforms {
        linux_x86_64: PlatformDetails {
            signature: "".to_string(),
            url: "".to_string(),
            name: Some("linux_x86_64".to_string()),
        },
        windows_x86_64: PlatformDetails {
            signature: "".to_string(),
            url: "".to_string(),
            name: Some("windows_x86_64".to_string()),
        },
        darwin_x86_64: PlatformDetails {
            signature: "".to_string(),
            url: "".to_string(),
            name: Some("darwin_x86_64".to_string()),
        },
        darwin_aarch64: PlatformDetails {
            signature: "".to_string(),
            url: "".to_string(),
            name: Some("darwin_aarch64".to_string()),
        },
    };

    let mut entries = fs::read_dir(kiosk_directory.clone()).await?;
    let mut versions = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_dir() {
            if let Some(folder_name) = path.file_name().and_then(|n| n.to_str()) {
                if let Ok(ver) = folder_name.parse::<Version>() {
                    versions.push((ver, folder_name.to_string()));
                }
            }
        }
    }

    // Sort in descending order (latest first)
    versions.sort_by(|a, b| b.0.cmp(&a.0));
    let version_names: Vec<String> = versions.into_iter().map(|(_, name)| name).collect();

    for version in version_names.iter() {
        let latest_folder = format!("{}/{}", kiosk_directory.clone(), version);
        // count platform total
        let platform_amount = platforms.iter().count();
        let mut platform_amount_counter = 0;
        for platform in platforms.iter_mut() {
            let platform_name = match platform.name.clone() {
                Some(name) => name,
                None => {
                    tracing::error!("failed to get platform name");
                    return Err(APIError::FileOrPathNotExist);
                }
            };

            let mut platforms_directory =
                match fs::read_dir(latest_folder.clone() + &String::from("/") + &platform_name)
                    .await
                    .inspect_err(|e| {
                        tracing::error!(
                            "failed to read directory: {}",
                            latest_folder.clone() + &String::from("/") + &platform_name
                        );
                    }) {
                    Ok(entries) => entries,
                    Err(e) => {
                        // Handle the error here
                        tracing::error!("failed to read directory: {}", e);
                        return Err(APIError::FileOrPathNotExist);
                    }
                };

            // checking file inside platform directory
            let mut is_platform_folder_not_empty = false;
            let mut is_signature_exist = false;
            let mut is_downladble_file_exist = false;
            while let Some(entry) = platforms_directory.next_entry().await? {
                let metadata = entry.metadata().await?;
                modified_date = metadata.created().or_else(|_| metadata.modified())?;
                let path = entry.path();
                // checking signature file
                if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("sig") {
                    // Read the content as string
                    let content = fs::read_to_string(path.display().to_string())
                        .await
                        .map_err(|_| {
                            tracing::error!(
                                "failed to read file: {}",
                                latest_folder.clone() + &String::from("/") + &platform_name
                            );
                            return APIError::FileOrPathNotExist;
                        })?;
                    platform.signature = content;
                    is_signature_exist = true;
                }

                // checking file besides sig extension
                if path.is_file() && path.extension().and_then(|e| e.to_str()) != Some("sig") {
                    platform.url = format!(
                        "{}/download/{}/{}/{}",
                        kiosk_url,
                        version,
                        platform_name,
                        path.file_name()
                            .and_then(|s| s.to_str())
                            .map_or("".to_string(), |s| s.to_string())
                    );
                    is_downladble_file_exist = true;
                }
                is_platform_folder_not_empty = is_signature_exist && is_downladble_file_exist;
            }
            if is_platform_folder_not_empty {
                platform_amount_counter += 1;
            }
        }
        if platform_amount == platform_amount_counter {
            let dt: chrono::DateTime<Utc> = modified_date.into();
            let pub_date = dt.to_rfc3339();
            return Ok(Json(KioskVersionResponse {
                version: version.to_string(),
                notes: "ini notes".to_string(),
                pub_date: pub_date.to_string(),
                platforms: platforms,
            }));
        }
    }

    Ok(Json(KioskVersionResponse {
        version: "".to_string(),
        notes: "ini notes".to_string(),
        // pub_date: kiosk_version.created_at.to_rfc3339(),
        pub_date: "1970-01-01T00:00:00+00:00".to_string(),
        platforms: platforms,
    }))
}

async fn download_file(
    Path((version, platform, filename)): Path<(String, String, String)>,
) -> Result<Response<Body>, APIError> {
    let kiosk_directory = dotenv::var("KIOSK_DIRECTORY").unwrap();
    let path = std::path::Path::new(&kiosk_directory)
        .join(&version)
        .join(&platform)
        .join(&filename);

    // // Check if file exists
    if !path.clone().exists() {
        return Err(APIError::NotFound);
    }

    let mime_type = mime_guess::from_path(&path).first_or_octet_stream();
    let file = tokio::fs::File::open(path)
        .await
        .inspect_err(|e| tracing::error!("failed to open file: {:?}", e))?;
    let stream = tokio_util::io::ReaderStream::new(file);

    let mut headers = HeaderMap::new();
    // headers.insert(header::CONTENT_TYPE, mime_type.as_ref().parse().unwrap());
    headers.insert(
        header::CONTENT_TYPE,
        mime_type.as_ref().parse().map_err(|e| {
            tracing::error!("failed to parse mime type {}", e);
            APIError::Internal
        })?,
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}\"", filename)
            .parse()
            .map_err(|e| {
                tracing::error!("failed to parse content disposition {}", e);
                APIError::Internal
            })?,
    );

    let mut response = Response::new(Body::from_stream(stream));
    *response.headers_mut() = headers;

    Ok(response)
}
