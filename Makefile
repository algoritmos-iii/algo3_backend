DOMAIN?=localhost
PORT?=8080
GROUP?=0
HELPER?=Ayudante

run:
	cargo run --release -- --port=${PORT}

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
