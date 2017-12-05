# Load .env file if it exists
-include .env
export

build:
	docker run --rm -v $(CURDIR):/usr/src/ci -w /usr/src/ci rust cargo build --release
.PHONY: build

image:
	docker build -t mre0/ci -f Dockerfile_workaround_build .
.PHONY: image

push:
	docker push mre0/ci
.PHONY: push

deploy:
	cd deploy && now --public -e GITHUB_TOKEN=${GITHUB_TOKEN}
.PHONY: deploy
