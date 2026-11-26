package main

import (
	"bufio"
	"fmt"
	"os"
)

// recWriter writes case records. Each field is encoded as
//
//	@<key> <byte length>\n<raw bytes>\n
//
// and a record ends with the line "@@". Raw bytes may contain anything
// (newlines, escape sequences), so no escaping is needed.
type recWriter struct {
	f     *os.File
	w     *bufio.Writer
	count int
	bytes int
}

func newRecWriter(path string) *recWriter {
	f, err := os.Create(path)
	if err != nil {
		panic(err)
	}
	return &recWriter{f: f, w: bufio.NewWriterSize(f, 1<<20)}
}

// rec writes one record from key/value pairs.
func (r *recWriter) rec(kv ...string) {
	if len(kv)%2 != 0 {
		panic("odd kv")
	}
	for i := 0; i < len(kv); i += 2 {
		fmt.Fprintf(r.w, "@%s %d\n%s\n", kv[i], len(kv[i+1]), kv[i+1])
		r.bytes += len(kv[i+1])
	}
	r.w.WriteString("@@\n")
	r.count++
}

func (r *recWriter) close() {
	if err := r.w.Flush(); err != nil {
		panic(err)
	}
	if err := r.f.Close(); err != nil {
		panic(err)
	}
	fmt.Fprintf(os.Stderr, "  %s: %d records, %d payload bytes\n", r.f.Name(), r.count, r.bytes)
}
