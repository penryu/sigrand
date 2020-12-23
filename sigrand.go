package main

import (
	"bufio"
	"fmt"
	"io/ioutil"
	"log"
	"math/rand"
	"os"
	"os/signal"
	"path"
	"strings"
	"syscall"
	"time"
)

var home = os.Getenv("HOME")
var sigFile = home + "/.sigfile"
var fifo = home + "/.signature"
var pidFile = home + "/.sigrandpid"

func cleanup() {
	os.Remove(pidFile)
}

func readSignatures(sigfile string) chan string {
	c := make(chan string)

	go func() {
		// open signature file
		fin, err := os.Open(sigfile)
		if err != nil {
			log.Fatal(err.Error() + `: ` + sigfile)
			return
		}
		defer fin.Close()

		// create string builder and grow to reasonable size
		var sb strings.Builder
		sb.Grow(512)

		// create scanner on sigfile, line-based by default
		scanner := bufio.NewScanner(fin)
		for scanner.Scan() {
			// if we reach our signature separator
			if line := scanner.Text(); line == "%%" {
				c <- sb.String() // write it to the channel
				sb.Reset()       // and clear it for the next iteration
			} else {
				// else, write the text to our builder, restoring the EOL
				sb.WriteString(line)
				sb.WriteByte('\n')
			}
		}

		close(c)
	}()

	return c
}

func setup() {
	// check for signature fifo
	fifoinfo, err := os.Stat(fifo)
	if err != nil {
		log.Fatalf("Pipe not found! Try `mkfifo -m u=rw '%s'`", fifo)
	} else if fifoinfo.Mode()&os.ModeNamedPipe == 0 {
		base := path.Base(fifo)
		log.Fatalf("%s must be a named pipe! Try `mkfifo -m u=rw '%s'`", base, fifo)
	}

	// check for extant pidfile
	if _, err := os.Stat(pidFile); err == nil {
		log.Fatalf("Found pidfile %s; exiting...", pidFile)
	}

	// write current pid to pidfile
	pidFileContent := []byte(fmt.Sprintln(os.Getpid()))
	if err := ioutil.WriteFile(pidFile, pidFileContent, 0600); err != nil {
		log.Fatalf("Error writing pidfile %s\n", pidFile)
	}

	// setup channel to catch keyboard interrupt (^C)
	c := make(chan os.Signal)
	signal.Notify(c, os.Interrupt, syscall.SIGTERM)
	go func() {
		<-c       // block for an intercepted signal
		cleanup() // perform teardown tasks
		os.Exit(0)
	}()
}

func main() {
	setup()
	defer cleanup()

	rng := rand.New(rand.NewSource(time.Now().UnixNano()))
	var signature string

	for {
		// block waiting for a signature reader
		log.Printf("Opening %s", fifo)
		fout, err := os.OpenFile(fifo, os.O_WRONLY, 0600)
		if err != nil {
			log.Fatal(err)
		}

		lineno := 0
		var evals strings.Builder
		// Randomly selects quotes with equal probability in one pass.
		for item := range readSignatures(sigFile) {
			lineno++
			if rng.Intn(lineno) < 1 {
				evals.WriteRune('+')
				signature = item
			} else {
				evals.WriteRune('-')
			}
		}

		log.Print(evals.String())
		log.Printf("Writing quote")
		fout.WriteString(signature)
		fout.Close()
		log.Print("Starting over...")
		time.Sleep(200 * time.Millisecond)
	}
}
