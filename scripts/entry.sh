#!/bin/bash

if [[ ! -x "/opt/aws-lambda-post-runner" ]]; then
    echo "Error: /opt/aws-lambda-post-runner not found or not executable" >&2
    exit 1
fi

exec /opt/aws-lambda-post-runner "$@"