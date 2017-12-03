build:
	docker run --rm -v $(CURDIR):/usr/src/ci -w /usr/src/ci rust cargo build --release

image:
	docker build -t mre0/ci -f Dockerfile_workaround_build .

push:
	docker push mre0/ci
