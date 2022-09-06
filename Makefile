DOMAIN?=localhost
PORT?=8080
GROUP?=0
HELPER?=Ayudante
FROM?=${PORT}
PADRON?=106223
EMAIL?=ilitteri@fi.uba.ar
SPREADSHEET_ID?=1jWXRFLamVmuAyTpv-n6737ze-8sgoAv1ZzHdyFXn4Rg
HELPSHEET_ID?=145qVyafYthG1dfCjbz-VcoABRqTkyGszqWK03Ax0L8A
CALENDAR_ID?=oeqsr7o5ftae7dav642rism2a4%40group.calendar.google.com

run:
	cargo run --release -- --domain=${DOMAIN} --port=${PORT} --spreadsheet-id=${SPREADSHEET_ID} --helpsheet-id=${HELPSHEET_ID}

test:
	cargo test

test_enqueue_help:
	curl --location --request POST "${DOMAIN}:${PORT}/api/discord/v1/enqueue_help" -H "Content-Type: application/json" -d '{"group": ${GROUP}, "voice_channel": 887022804183175188}'

test_get_next:
	curl --location --request GET "${DOMAIN}:${PORT}/api/discord/v1/next" -H 'Content-Type: application/json' -d '"${HELPER}"'

test_dismiss:
	curl --location --request POST "${DOMAIN}:${PORT}/api/discord/v1/dismiss_help" -H 'Content-Type: application/json' -d '${GROUP}'

test_clear:
	curl --location --request PATCH "${DOMAIN}:${PORT}/api/discord/v1/clear_help_queue"

test_get_queue:
	curl --location --request GET "${DOMAIN}:${PORT}/api/discord/v1/help_queue"

test_is_student:
	curl --location --request GET "${DOMAIN}:${PORT}/api/discord/v1/is_student" -H "Content-Type: application/json" -d '{"id": ${PADRON}, "email": "${EMAIL}"}'

test_get_group:
	curl --location --request GET "${DOMAIN}:${PORT}/api/discord/v1/group" -H "Content-Type: application/json" -d '{"id": ${PADRON}, "email": "${EMAIL}"}'

test_get_next_class:
	curl --location --request GET "${DOMAIN}:${PORT}/api/discord/v1/next_class"

build_docker:
	docker build -t algo3_backend .

run_docker: build_docker
	docker run --rm -p ${FROM}:80 -d algo3_backend
