# pregonero

[![container](https://github.com/darioblanco/pregonero/actions/workflows/container.yaml/badge.svg)](https://github.com/darioblanco/pregonero/actions/workflows/container.yaml)
[![json](https://github.com/darioblanco/pregonero/actions/workflows/json.yaml/badge.svg)](https://github.com/darioblanco/pregonero/actions/workflows/json.yaml)
[![test](https://github.com/darioblanco/pregonero/actions/workflows/test.yaml/badge.svg)](https://github.com/darioblanco/pregonero/actions/workflows/test.yaml)
[![validate](https://github.com/darioblanco/pregonero/actions/workflows/validate.yaml/badge.svg)](https://github.com/darioblanco/pregonero/actions/workflows/validate.yaml)
[![yaml](https://github.com/darioblanco/pregonero/actions/workflows/yaml.yaml/badge.svg)](https://github.com/darioblanco/pregonero/actions/workflows/yaml.yaml)

Rust backend to connect to IMAP servers and process new email messages into a queue.

A "pregonero" is a term from Spanish that translates to "town crier" in English.
Historically, town criers were officials appointed by the local authorities to make
public announcements in the streets. Prior to the advent of literacy and later on,
newspapers, town criers were the primary means of news communication with the townspeople,
since many were illiterate.

In the context of an email application, a "pregonero" would be a fitting metaphor
for a service that notifies you of new messages or news, acting as a modern,
digital equivalent of the town crier.

And well... This Rust playground just needed a name.

## Development

Test

```sh
make test
```

Build

```sh
make build
```

Run

```sh
make run
```

Watch

```sh
make watch
```
