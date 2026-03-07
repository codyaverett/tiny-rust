# 31-tiny-smtp

A minimal SMTP server in `no_std` Rust. Implements a line-oriented text protocol state machine that accepts email messages and logs them to stdout.

## What it does

- Listens on port 2525 (unprivileged, no root needed)
- Implements minimal SMTP: EHLO, HELO, MAIL FROM, RCPT TO, DATA, QUIT, RSET, NOOP
- Logs received emails to stdout (sender, recipient, data size)
- Responds with proper SMTP status codes (220, 250, 354, 221, 500, 503)

## New concepts

- **Text-based protocol parsing** -- line-oriented, CR+LF terminated
- **SMTP state machine** -- INIT -> GREETED -> MAIL -> RCPT -> DATA
- **Multi-line protocol responses** -- EHLO capabilities list
- **Dot-stuffing termination** -- message body ends with `\r\n.\r\n`
- First non-HTTP application protocol in the series

## SMTP protocol flow

```
Server: 220 tiny-smtp ready\r\n
Client: EHLO hostname\r\n
Server: 250-tiny-smtp\r\n250-SIZE 10240\r\n250 OK\r\n
Client: MAIL FROM:<sender@example.com>\r\n
Server: 250 OK\r\n
Client: RCPT TO:<rcpt@example.com>\r\n
Server: 250 OK\r\n
Client: DATA\r\n
Server: 354 Start mail input; end with <CRLF>.<CRLF>\r\n
Client: Subject: test\r\n\r\nHello\r\n.\r\n
Server: 250 OK\r\n
Client: QUIT\r\n
Server: 221 Bye\r\n
```

## State machine

```
INIT  --EHLO/HELO-->  GREETED
GREETED  --MAIL FROM-->  MAIL
MAIL  --RCPT TO-->  RCPT
RCPT  --RCPT TO-->  RCPT  (multiple recipients)
RCPT  --DATA-->  DATA  (read until \r\n.\r\n, then -> GREETED)
any  --RSET-->  GREETED
any  --QUIT-->  close
any  --NOOP-->  250 OK (no state change)
wrong sequence -->  503 Bad sequence
unknown cmd -->  500 Unrecognized
```

## Usage

```sh
cargo build --release
./target/release/tiny-smtp &
# SMTP server listening on port 2525
```

### Testing with swaks

```sh
swaks --to user@example.com --from sender@test.com --server localhost:2525
```

### Testing with telnet/nc

```sh
echo -e "EHLO test\r\nMAIL FROM:<a@b.com>\r\nRCPT TO:<c@d.com>\r\nDATA\r\nHello\r\n.\r\nQUIT\r\n" | nc localhost 2525
```

### Server output

```
[#1] MAIL FROM:<a@b.com> TO:<c@d.com> (6 bytes)
```

## Limitations

- Port 2525 (not 25, avoids root)
- No TLS/STARTTLS
- No actual mail delivery -- logs to stdout only
- Max 10KB message size
- Single-threaded, one connection at a time
- Case-insensitive command matching (simple ASCII lowering)
