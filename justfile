set dotenv-load

export PATH := "./node_modules/.bin:" + env_var('PATH')

default:
    just --list

test:
    cargo nextest run --no-capture

