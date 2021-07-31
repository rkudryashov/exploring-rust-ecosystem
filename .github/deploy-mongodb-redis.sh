#!/bin/bash
shopt -s expand_aliases
alias docker-compose='docker run --rm \
    -v /var/run/docker.sock:/var/run/docker.sock \
    -v "$PWD:$PWD" \
    -w="$PWD" \
    docker/compose:1.29.2'

cd ~/mongodb-redis

docker system prune
docker-compose pull
docker-compose down
docker-compose up -d
