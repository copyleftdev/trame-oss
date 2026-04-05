# trame

The X12 EDI Swiss Army knife.

## Install

```sh
cargo install trame-cli
```

## Commands

| Command | Description |
|---------|-------------|
| `fmt` | Pretty-print X12 with one segment per line, indented by envelope depth |
| `info` | Show interchange, group, and transaction set summary |
| `help` | Show usage |
| `version` | Show version |

## Usage

### fmt -- Pretty-print

```sh
$ trame fmt claim.edi
ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:~
  GS*HP*SENDER*RECEIVER*20210901*1234*1*X*005010X222A1~
    ST*837*0001*005010X222A1~
      BHT*0019*00*12345*20210901*1234*CH~
      CLM*PATIENT1*100***11:B:1*Y*A*Y*I~
    SE*4*0001~
  GE*1*1~
IEA*1*000000001~
```

Reads from a file or stdin:

```sh
cat claim.edi | trame fmt
```

### info -- Interchange summary

```sh
$ trame info claim.edi
Interchange: 000000001
  Sender:   ZZ/SENDER
  Receiver: ZZ/RECEIVER
  Date:     210901 1234
  Version:  00401
  Usage:    P (Production)
  Groups:   1
    Group 1: HP (Health Care Claim Payment/Advice) v005010X222A1
      Control: 1
      Transactions: 1
        [1] 837 Health Care Claim -- 4 segments
```

## Pro Commands

Additional commands are available with `trame-pro`:

`validate`, `to-json`, `from-json`, `fake`, `receive`, `status`, `search`

See [github.com/copyleftdev/trame](https://github.com/copyleftdev/trame) for details.

## Part of trame

`trame-cli` is the command-line interface for the [trame](https://github.com/copyleftdev/trame) workspace.

## License

Apache-2.0
