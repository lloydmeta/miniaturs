start_dev_env:
	cd terraform/localdev && docker-compose up &

init_dev_env:
	cd terraform/localdev && tflocal init

provision_dev_env:
	cd terraform/localdev && tflocal apply -auto-approve

clean_dev_env:
	rm -rf terraform/localdev/volume

stop_dev_env:
	cd terraform/localdev && docker-compose down

begin_dev:
	source dev.env && cd server && cargo lambda watch

prod_workspace:
	@cd terraform/prod && terraform workspace select miniaturs

provision_prod: prod_workspace
	@cd terraform/prod && terraform apply -auto-approve

plan_prod: prod_workspace
	@cd terraform/prod && terraform plan

signature_for_dev:
	@echo "http://localhost:9000/$$(source dev.env && cd client && cargo run -- $(TO_SIGN))"

signature_for_localstack:
	@echo "$$(cd terraform/localdev && tflocal output --raw lambda_function_url)$$(export MINIATURS_SHARED_SECRET=$$(cd terraform/localdev && tflocal output --raw miniaturs_shared_secret) && cd client && cargo run -- $(TO_SIGN))"

signature_for_prod:
	@echo "$$(cd terraform/prod && terraform output --raw miniaturs_deployed_url)/$$(export MINIATURS_SHARED_SECRET=$$(cd terraform/prod && terraform output --raw miniaturs_shared_secret) && cd client && cargo run -- $(TO_SIGN))"

format:
	@cd terraform && terraform fmt -recursive
	@cargo fmt --all
	@cd client && cargo fmt