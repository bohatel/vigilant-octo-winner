#!/usr/bin/env bash
set -x
set -eo pipefail

docker stop zero2prod_db && docker rm zero2prod_db
