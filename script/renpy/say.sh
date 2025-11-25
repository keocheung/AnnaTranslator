#!/usr/bin/env bash

curl -X POST http://127.0.0.1:17889/submit --data-raw "$1"
