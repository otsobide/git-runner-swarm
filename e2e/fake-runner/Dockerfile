# Stands in for GitHub's official runner image in the end-to-end tests:
# laid out like it (a `runner` user, config.sh and run.sh in its home)
# so the panel's own registration script runs in it unchanged, but with
# scripts that write down how they were called instead of talking to
# GitHub. No token, account or network is needed, and none is used.
FROM alpine:3.22
RUN apk add --no-cache bash \
 && adduser -D -u 1001 runner
WORKDIR /home/runner
COPY --chown=runner:runner --chmod=0755 config.sh run.sh ./
USER runner
CMD ["bash"]
