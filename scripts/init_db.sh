#!/usr/bin/env bash
set -x
set -eo pipefail

if ! [ -x "$(command -v psql)" ]; then
    echo >&2 "Error: psql is not installed."
    exit 1
fi

if ! [ -x "$(command -v sqlx)" ]; then
    echo >&2 "Error: sqlx is not installed."
    echo >&2 "Use:"
    echo >&2 " cargo install sqlx-cli --no-default-features --features rustls,postgres"
    echo >&2 "to install it."
    exit 1
fi

CONTAINER_NAME="zero2prod_db"

DB_USER=${POSTGRES_USER:=postgres}
DB_PASSWORD="${POSTGRES_PASSWORD:=password}"
DB_NAME="${POSTGRES_DB:=newsletter}"
DB_HOST="${POSTGRES_HOST:=localhost}"
DB_PORT="${POSTGRES_PORT:=5435}"

export PGPASSWORD="${DB_PASSWORD}"

if [[ -z "${SKIP_DOCKER}" ]]
then
    if [ "$(docker ps -aq -f name=${CONTAINER_NAME})" ]; then
        >&2 echo "Postgres container already running"
    else
        docker run \
            --name ${CONTAINER_NAME} \
            -e POSTGRES_USER=${DB_USER} \
            -e POSTGRES_PASSWORD=${DB_PASSWORD} \
            -e POSTGRES_DB=${DB_NAME} \
            -p "${DB_PORT}":5432 \
            -d postgres \
            postgres -N 1000
    fi
fi

# Keep pinging Postgres until it's ready to accept commands
i=0
until psql -h "${DB_HOST}" -U "${DB_USER}" -p "${DB_PORT}" -d "postgres" -c '\q'; do
    >&2 echo "Postgres is still unavailable - sleeping"
    if [[ $i -lt 15 ]]
    then
          ((i=i+1))
          sleep 1
    else
        >&2 echo "Giving up on Postgres! It did not respond in ${i} seconds"
        exit 1
    fi
done
>&2 echo "Postgres is up and running on port ${DB_PORT}!"

DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}
export DATABASE_URL
sqlx database create
sqlx migrate run

>&2 echo "Postgres migrations applied, ready to go!"
