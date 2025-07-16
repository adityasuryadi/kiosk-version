use crate::{
    entity::{
        kiosk_version::{self, Model},
        kiosk_version_platform,
    },
    error::APIError,
};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, request, HeaderMap, Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    serve, Json, Router,
};
use dotenv::dotenv;
use sea_orm::{
    sqlx::types::chrono::{self, Utc},
    ActiveModelTrait,
    ActiveValue::{NotSet, Set},
    Database, DatabaseConnection, EntityTrait, QueryOrder, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::{fs::File, io, os::darwin, path::PathBuf, sync::Arc, time::SystemTime};
use tokio::{
    fs::{self},
    net::TcpListener,
};
use tracing::Level;
use tracing_subscriber::fmt::Subscriber;

mod entity;
mod error;

pub struct AppState {
    pub database: DatabaseConnection,
    pub public_path: PathBuf,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let app_url = "localhost:3000";
    let db_url = dotenv::var("DATABASE_URL").unwrap();
    let database = Database::connect(db_url).await.unwrap();
    let public_dir = std::path::Path::new("./public");
    if !public_dir.exists() {
        fs::create_dir(public_dir)
            .await
            .expect("Failed to create public directory");
    }
    let state = Arc::new(AppState {
        database,
        public_path: public_dir.to_path_buf(),
    });

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
        .route("/download/{filename}", get(download_file))
        .with_state(state);
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
    pub platforms: Platforms,
}

// TODO
// - [x] create versioning enpoint
// - [x] create folder base version name
// - [x] validate folder if exist
// - [] checking last created folder
// - [] checking isi folder
// - [] notes input ke txt

pub async fn create_kiosk_version(
    State(state): State<Arc<AppState>>,
    request: Json<CreateKioskVersionRequest>,
) -> Result<StatusCode, APIError> {
    let kiosk_directory = dotenv::var("KIOSK_DIRECTORY").unwrap();
    let mut dir = tokio::fs::read_dir(kiosk_directory.clone()).await?;
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

// #[derive(Serialize, Deserialize)]
// pub struct KioskVersionResponse {
//     pub version: String,
//     pub note: String,
//     pub url: String,
//     pub signature: String,
//     pub created_at: chrono::DateTime<Utc>,
//     pub updated_at: chrono::DateTime<Utc>,
// }

#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformDetails {
    pub signature: String,
    pub url: String,
    pub name: String,
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
// - [] check isi folder terbaru
// - [] jika isi folder terbaru kosong maka return folder terbaru yang ada isinya

pub async fn get_latest_version(
    State(state): State<Arc<AppState>>,
) -> Result<Json<KioskVersionResponse>, APIError> {
    let mut newest_folder: Option<(String, SystemTime)> = None;
    let mut newewst_folder_name = "".to_string();
    let kiosk_directory = dotenv::var("KIOSK_DIRECTORY").unwrap();
    let mut dir = tokio::fs::read_dir(kiosk_directory.clone()).await?;
    let mut modified_date: SystemTime = SystemTime::UNIX_EPOCH;
    let kiosk_url = dotenv::var("KIOSK_DOWNLOADABLE_URL").unwrap();
    // let platforms: Vec<String> = vec![
    //     "windows_x86_64".to_string(),
    //     "linux_x86_64".to_string(),
    //     "darwin_x86_64".to_string(),
    //     "darwin_aarch64".to_string(),
    // ];

    let mut platforms = Platforms {
        linux_x86_64: PlatformDetails {
            signature: "".to_string(),
            url: "".to_string(),
            name: "linux_x86_64".to_string(),
        },
        windows_x86_64: PlatformDetails {
            signature: "".to_string(),
            url: "".to_string(),
            name: "windows_x86_64".to_string(),
        },
        darwin_x86_64: PlatformDetails {
            signature: "".to_string(),
            url: "".to_string(),
            name: "darwin_x86_64".to_string(),
        },
        darwin_aarch64: PlatformDetails {
            signature: "".to_string(),
            url: "".to_string(),
            name: "darwin_aarch64".to_string(),
        },
    };

    while let Some(entry) = dir.next_entry().await? {
        let path = entry.path();
        // Check if it's a directory
        if entry.file_type().await?.is_dir() {
            // Get modification time
            let metadata = entry.metadata().await?;
            let modified = metadata.modified()?;

            modified_date = metadata.created().or_else(|_| metadata.modified())?;

            // Update newest folder if this one is newer
            match newest_folder {
                Some((_, current_time)) if modified > current_time => {
                    newest_folder = Some((path.display().to_string(), modified));
                    newewst_folder_name = match path.file_name().and_then(|f| f.to_str()) {
                        Some(name) => name.to_string(),
                        None => String::new(),
                    };
                }
                None => {
                    newest_folder = Some((path.display().to_string(), modified));
                    newewst_folder_name = match path.file_name().and_then(|f| f.to_str()) {
                        Some(name) => name.to_string(),
                        None => String::new(),
                    };
                }
                _ => {}
            }
        }
    }

    let mut latest_folder = "".to_string();
    if let Some((folder, _)) = newest_folder {
        latest_folder = folder;
    } else {
        println!("No directories found");
    }

    println!("Latest folder: {}", latest_folder);
    // read file in platforms folder
    // for platform in platforms.clone() {
    //     let mut platform_entries =
    //         fs::read_dir(latest_folder.clone() + &String::from("/") + &platform).await?;
    //     println!("Platform: {}", platform);
    //     while let Some(entry) = platform_entries.next_entry().await? {
    //         let path = entry.path();

    //         if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("sig") {
    //             // Read the content as string
    //             let content = fs::read_to_string(path.display().to_string())
    //                 .await
    //                 .map_err(|_| {
    //                     tracing::error!(
    //                         "failed to read file: {}",
    //                         latest_folder.clone() + &String::from("/") + &platform
    //                     );
    //                     return APIError::Internal;
    //                 })?;
    //             println!("Signature content:\n{}", content);
    //         }
    //     }
    // }

    // for (name, details) in platforms.iter() {
    //     let mut platform_entries =
    //         fs::read_dir(latest_folder.clone() + &String::from("/") + &name).await?;
    //     while let Some(entry) = platform_entries.next_entry().await? {
    //         let path = entry.path();

    //         if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("sig") {
    //             // Read the content as string
    //             let content = fs::read_to_string(path.display().to_string())
    //                 .await
    //                 .map_err(|_| {
    //                     tracing::error!(
    //                         "failed to read file: {}",
    //                         latest_folder.clone() + &String::from("/") + &name
    //                     );
    //                     return APIError::Internal;
    //                 })?;
    //         }
    //     }
    // }

    // platforms.iter_mut().for_each(|platform| {
    //     let platform_name = serde_json::to_string(platform)
    //         .map_err(|e| tracing::error!("{}", e))
    //         .unwrap();
    //     let mut platform_entries =
    //         fs::read_dir(latest_folder.clone() + &String::from("/") + &platform_name).await?;
    //     while let Some(entry) = platform_entries.next_entry().await? {
    //         let path = entry.path();

    //         if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("sig") {
    //             // Read the content as string
    //             let content = fs::read_to_string(path.display().to_string())
    //                 .await
    //                 .map_err(|_| {
    //                     tracing::error!(
    //                         "failed to read file: {}",
    //                         latest_folder.clone() + &String::from("/") + &platform_name
    //                     );
    //                     return APIError::Internal;
    //                 })?;
    //             platform.signature = content;
    //         }
    //     }
    // });

    for platform in platforms.iter_mut() {
        let platform_name = platform.name.clone().to_string();
        let mut platform_entries =
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
                    return Err(APIError::Internal);
                }
            };
        while let Some(entry) = platform_entries.next_entry().await? {
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("sig") {
                // Read the content as string
                let content = fs::read_to_string(path.display().to_string())
                    .await
                    .map_err(|_| {
                        tracing::error!(
                            "failed to read file: {}",
                            latest_folder.clone() + &String::from("/") + &platform_name
                        );
                        return APIError::Internal;
                    })?;
                platform.signature = content;
            }

            if path.is_file() && path.extension().and_then(|e| e.to_str()) != Some("sig") {
                platform.url = format!(
                    "{}/{}/{}/{}",
                    kiosk_url,
                    newewst_folder_name,
                    platform_name,
                    path.file_name()
                        .and_then(|s| s.to_str())
                        .map_or("".to_string(), |s| s.to_string())
                );
            }
        }
    }

    let dt: chrono::DateTime<Utc> = modified_date.into();
    let pub_date = dt.to_rfc3339();

    Ok(Json(KioskVersionResponse {
        version: newewst_folder_name,
        notes: "ini notes".to_string(),
        // pub_date: kiosk_version.created_at.to_rfc3339(),
        pub_date: pub_date.to_string(),
        platforms: platforms,
    }))
}

async fn download_file(
    State(state): State<Arc<AppState>>,
    Path(filename): Path<String>,
) -> Result<Response<Body>, APIError> {
    let path = state.public_path.join(&filename);

    // Check if file exists
    if !path.exists() {
        return Err(APIError::NotFound);
    }

    // Get the file's mime type for content-type header
    // let mime_type = mime_guess::from_path(&path)
    //     .first_or_octet_stream();

    let mime_type = mime_guess::from_path(&path).first_or_octet_stream();

    // let file = File::open(&path).await?;
    let file = tokio::fs::File::open(path)
        .await
        .inspect_err(|e| tracing::error!("failed to open file: {:?}", e))?;
    // let stream = tokio_util::io::ReaderStream::new(file);
    let stream = tokio_util::io::ReaderStream::new(file);

    // let headers = [
    //     (header::CONTENT_TYPE, mime_type.as_ref()),
    //     (
    //         header::CONTENT_DISPOSITION,
    //         &format!("attachment; filename=\"{}\"", filename),
    //     ),
    // ];

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, mime_type.as_ref().parse().unwrap());
    headers.insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}\"", filename)
            .parse()
            .unwrap(),
    );

    // // Ok((headers, stream))
    // Response::builder()
    //     .status(StatusCode::OK)
    //     .headers(headers)
    //     .body(Body::from(stream))

    let mut response = Response::new(Body::from_stream(stream));
    *response.headers_mut() = headers;

    Ok(response)
}
