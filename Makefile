
NAME ?=

.PHONY: migrate

migrate:
ifeq ($(strip $(NAME)),)
	@echo "❌ Please provide a migration name: make migrate NAME=your_migration_name"
	@exit 1
else
	@sea-orm-cli migrate generate $(NAME)
endif
	
migrate-up-dev:
	set +H; \
	sea-orm-cli migrate up  -u mysql://root:root@localhost/versioning

migrate-down-dev:
	set +H; \
	sea-orm-cli migrate down  -u mysql://root:root@localhost/versioning