# awesome-static-analysis-ci

This is the continuous integration framework for [mre/awesome-static-analyis](https://github.com/mre/awesome-static-analysis/).

### Deployment

Currently, this tool is deployed on [`zeit.co`](http://zeit.co/).  
There are a few limitations, that's why the deploy process could be more straightforward.  


First, build a new version of the binary:

```
make build # Build binary
make image # Copy binary to Docker image
make push  # Push Docker image to Dockerhub
```

After that, deploy the new version using zeit's `now` tool.  
We do this from a `deploy` subdirectory, to avoid copying the full (>200 MB) build context to zeit.  
Inside the subdirectory, there's just a Dockerfile, which references our newly built Docker image.  
We need to specify the Github token to be able to set status reports for our pull requests.  

```
cd deploy && now -e GITHUB_TOKEN=<TOKEN>
```

Note:
As of now, zeit does not support multistage builds (see [Issue 962](https://github.com/zeit/now-cli/issues/962)).
Once this is possible, `Dockerfile_multistage_issue_962` can replace the `make` process.