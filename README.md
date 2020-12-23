# NAME

`sigrand.go` -- a Go port of [tomc][tomc]'s classic `sigrand.pl`

# DESCRIPTION

The format of the [sigfile](./sigfile) takes advantage of Perl's `$/` special
var, which allows you to change the line separator when reading from filehandles
using the `<>` operator.

Since Go doesn't offer this syntax sugar, the file is read in a coroutine and
complete, `%%`-separated records are pushed through a `chan string` to be
iterated over with `range` in the reader logic.

The `%%` record separator is currently hard-coded, but could be trivially
factored out, though the scanner is still pretty line-oriented. Exercise for the
reader!

# OVERVIEW

The original version of the script functioned somewhat like a daemon and was
often invoked and immediately backgrounded in a `.bashrc` or similar startup
script.

However, I've added a bit of logging (if only to analyze the quote selection
logic) and it could be a bit noisy of not detached from stdout. Nevertheless,
the operation is deceptively simple:

1. `sigrand` loads, ...
  1. ... looks for a pidfile
  1. ... checks for a Unix [named pipe][fifodoc] at `$HOME/.signature`
  1. ... opens the pipe, blocking and waiting for someone to read
1. When a process reads from the pipe, `sigrand` ...
  1. ... opens the sigfile ("database" of signatures) at `$HOME/.sigfile`
  1. ... iterates over each record, selecting one at random
  1. ... writes the selected record to the pipe
  1. ... closes the pipe
  1. ... waits a split second for the reader to close as well
  1. ... repeats

To stop the `sigrand` process, you have two options:

- Send a keyboard interrupt (`^C`) on the process's `stdin`
- send a `TERM` signal to the pid in `$HOME/.sigrandpid`


# EXAMPLE

## First Time Setup

```
$ go build && cp sigrand /usr/local/bin
$ cp sigfile ~/.sigfile
$ mkfifo -m 600 ~/.signature ./sigrand
```

## In Regular Use

In one shell session:

```
$ sigrand
2020/12/23 00:06:32 Opening /Users/penryu/.signature
2020/12/23 00:06:34 ++-----+---------+--------------------------------+------------
2020/12/23 00:06:34 Writing quote
2020/12/23 00:06:34 Starting over...
2020/12/23 00:06:34 Opening /Users/penryu/.signature
```

In other shells or processes, simply read from `~/.signature`:

```
$ cat ~/.signature
"What duck?"
        -- (Terry Pratchett, Soul Music)
$
```

(The default sigfile is just a bunch of Terry Pratchett quotes.)

See [start.sh](./start.sh) for another example use.

# THE ORIGINAL

The original can be found in [The Perl Cookbook][perlcook], and versions of it
are scattered throughout the Interwebs.

# COPYRIGHT

&copy; 2020 Tim Hammerquist

This code is licensed under the terms of the [Artistic License](./LICENSE).

[tomc]: https://www.oreilly.com/pub/au/407
[perlcook]: https://www.oreilly.com/library/view/perl-cookbook/1565922433/

[fifodoc]: https://man7.org/linux/man-pages/man7/fifo.7.html
