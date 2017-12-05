# awesome-static-analysis-ci

This is the continuous integration framework for [mre/awesome-static-analyis](https://github.com/mre/awesome-static-analysis/).

### Deployment

Add your Github token to a `.env` file:

```
echo "GITHUB_TOKEN=<INSERT_TOKEN_HERE>" > .env
```

To deploy, simply run the following command:

```
make deploy
```

Finally, set an alias for the newly deployed domain:

```
now ls
now alias set <hash-id>.now.sh check.now.sh
```

### Notes:

Currently, this tool is deployed on [`zeit.co`](http://zeit.co/).  
There are a few limitations, that's why the deploy process could be more straightforward.  

As of now, zeit does not support multistage builds (see [Issue 962](https://github.com/zeit/now-cli/issues/962)).
Once this is possible, `Dockerfile_multistage_issue_962` can replace the `make` process.

For now, we use a `deploy` subdirectory to avoid copying the full (>200 MB) build context to zeit.  
Inside the subdirectory, there's just a Dockerfile, which references our newly production Docker image.  
We need to specify the Github token to be able to set status reports for our pull requests.  
