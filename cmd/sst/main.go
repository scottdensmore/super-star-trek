package main

import (
	"os"

	"github.com/scottdensmore/super-star-trek/pkg/classic"
)

func main() {
	os.Exit(classic.RunClassicCLI(os.Stdin, os.Stdout, os.Args[1:]))
}
