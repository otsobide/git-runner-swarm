# Every test there is, run the way CI runs them.
#
#   make test           the backend's, the web panel's and the e2e tests
#   make test/backend   fmt, clippy, unit and integration tests (Docker's too)
#   make test/web       typecheck, unit and integration tests, build
#   make test/e2e       the panel's container driven in a browser, then the
#                       web panel's types held to the API it serves
#   make ci             the CI workflow itself, on this machine, with act
#
# E2E_BROWSER_CHANNEL=chrome runs the e2e tests in an installed Chrome
# rather than the Chromium Playwright downloads.

.PHONY: test test/backend test/web test/e2e ci

test: test/backend test/web test/e2e

test/backend:
	cd backend && cargo fmt --check
	cd backend && cargo clippy --workspace --all-targets --locked -- -D warnings
	cd backend && cargo test --workspace --locked -- --include-ignored

test/web:
	cd web && npm ci
	cd web && npm run typecheck
	cd web && npm test
	cd web && npm run build

# The panel is kept up between the e2e tests and the contract test, and
# stopped whatever either of them did.
test/e2e:
	cd e2e && npm ci
	cd web && npm ci
	status=0; \
	(cd e2e && E2E_KEEP=1 npm test) || status=1; \
	(cd web && GRS_API=http://127.0.0.1:18090/api npm run test:contract) || status=1; \
	(cd e2e && docker compose --file compose.yaml down); \
	exit $$status

ci:
	act --workflows .github/workflows/ci.yml \
	  --platform ubuntu-latest=catthehacker/ubuntu:act-latest
