// Command mutants puts mutations into Go source one at a time, runs a test
// command for each, and puts the file back. See ../SKILL.md.
//
//	mutants -file F [-lines 3,10-12] [-list] -- <test command>...
//	    the syntactic mutations of one file: comparison, logical and
//	    arithmetic operators turned around, "!" dropped, true and false
//	    swapped, integer literals off by one
//	mutants -spec LIST [-only TEXT] -- <test command>...
//	    the mutations written in LIST, one per line:
//	    name | file | old | new   (⏎ is a line feed, ⇥ a tab; # starts a comment)
//
// It runs in the current directory: run it in a copy of the repository, not in
// the working tree. Exit code: 0 no mutation survived, 1 some survived, 2 a
// mistake in the use, or the tests fail before any mutation.
package main

import (
	"bytes"
	"context"
	"errors"
	"flag"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"os"
	"os/exec"
	"os/signal"
	"strconv"
	"strings"
	"time"
)

// A mutation replaces the text of file from start to end with repl.
type mutation struct {
	name       string
	file       string
	start, end int
	repl       string
}

func main() {
	file := flag.String("file", "", "the Go file whose syntactic mutations to make")
	lines := flag.String("lines", "", "with -file: only the mutations on these lines (3,10-12)")
	list := flag.Bool("list", false, "only list the mutations")
	spec := flag.String("spec", "", "a file that lists the mutations to make")
	only := flag.String("only", "", "with -spec: only the mutations whose name holds this text")
	timeout := flag.Duration("timeout", 5*time.Minute, "the longest one run of the tests may take")
	flag.Parse()

	var ms []mutation
	var err error
	switch {
	case (*file == "") == (*spec == ""):
		err = errors.New("give -file or -spec")
	case *file != "":
		ms, err = syntactic(*file, *lines)
	default:
		ms, err = fromSpec(*spec, *only)
	}
	if err != nil {
		fmt.Fprintln(os.Stderr, "mutants:", err)
		os.Exit(2)
	}
	if *list {
		for _, m := range ms {
			fmt.Println(m.name)
		}
		return
	}
	if flag.NArg() == 0 {
		fmt.Fprintln(os.Stderr, "mutants: give the test command after --")
		os.Exit(2)
	}
	os.Exit(runAll(ms, flag.Args(), *timeout))
}

// runAll makes each mutation, runs cmd, and puts the file back, whatever
// happens (an interrupt included).
func runAll(ms []mutation, cmd []string, timeout time.Duration) int {
	if out, err := run(cmd, timeout); err != nil {
		fmt.Fprintf(os.Stderr, "mutants: the tests fail before any mutation, so nothing they say means anything:\n%s", out)
		return 2
	}
	var current struct {
		file string
		src  []byte
	}
	restore := func() {
		if current.file != "" {
			_ = os.WriteFile(current.file, current.src, 0o644)
			current.file = ""
		}
	}
	defer restore()
	sig := make(chan os.Signal, 1)
	signal.Notify(sig, os.Interrupt)
	go func() {
		<-sig
		restore()
		os.Exit(130)
	}()

	survived := 0
	for _, m := range ms {
		src, err := os.ReadFile(m.file)
		if err != nil {
			fmt.Println("error:", m.name, err)
			continue
		}
		if m.end > len(src) {
			fmt.Println("error:", m.name, "the file changed")
			continue
		}
		current.file, current.src = m.file, src
		mutated := append(append(append([]byte{}, src[:m.start]...), m.repl...), src[m.end:]...)
		if err := os.WriteFile(m.file, mutated, 0o644); err != nil {
			fmt.Println("error:", m.name, err)
			restore()
			continue
		}
		out, err := run(cmd, timeout)
		restore()
		switch {
		case err == nil:
			survived++
			fmt.Println("SURVIVED:", m.name)
		case notCompiled(out):
			fmt.Println("not compiled:", m.name, "(change the mutation: this says nothing about the tests)")
		default:
			fmt.Println("killed:", m.name, failing(out))
		}
	}
	fmt.Printf("survived %d of %d\n", survived, len(ms))
	if survived > 0 {
		return 1
	}
	return 0
}

func run(cmd []string, timeout time.Duration) ([]byte, error) {
	ctx, cancel := context.WithTimeout(context.Background(), timeout)
	defer cancel()
	return exec.CommandContext(ctx, cmd[0], cmd[1:]...).CombinedOutput()
}

func notCompiled(out []byte) bool {
	return bytes.Contains(out, []byte("[build failed]")) || bytes.Contains(out, []byte("[setup failed]"))
}

// failing is the names of the tests that failed, as go test prints them.
func failing(out []byte) []string {
	var names []string
	for _, l := range strings.Split(string(out), "\n") {
		if f := strings.Fields(strings.TrimSpace(l)); len(f) >= 3 && f[0] == "---" && f[1] == "FAIL:" {
			names = append(names, f[2])
		}
	}
	return names
}

var turned = map[token.Token]token.Token{
	token.EQL: token.NEQ, token.NEQ: token.EQL,
	token.LSS: token.LEQ, token.LEQ: token.LSS,
	token.GTR: token.GEQ, token.GEQ: token.GTR,
	token.LAND: token.LOR, token.LOR: token.LAND,
	token.ADD: token.SUB, token.SUB: token.ADD,
	token.MUL: token.QUO, token.QUO: token.MUL,
	token.INC: token.DEC, token.DEC: token.INC,
	token.ADD_ASSIGN: token.SUB_ASSIGN, token.SUB_ASSIGN: token.ADD_ASSIGN,
}

// syntactic lists the syntactic mutations of file, on the lines given (all
// lines if lines is empty), in the order of the source.
func syntactic(file, lines string) ([]mutation, error) {
	keep, err := lineFilter(lines)
	if err != nil {
		return nil, err
	}
	src, err := os.ReadFile(file)
	if err != nil {
		return nil, err
	}
	return syntacticSrc(file, src, keep)
}

func syntacticSrc(file string, src []byte, keep func(int) bool) ([]mutation, error) {
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, file, src, 0)
	if err != nil {
		return nil, err
	}
	var ms []mutation
	add := func(pos token.Pos, old, repl string) {
		p := fset.Position(pos)
		if keep(p.Line) {
			ms = append(ms, mutation{
				name: fmt.Sprintf("%s:%d: %s -> %s", file, p.Line, old, repl),
				file: file, start: p.Offset, end: p.Offset + len(old), repl: repl,
			})
		}
	}
	ast.Inspect(f, func(n ast.Node) bool {
		switch x := n.(type) {
		case *ast.GenDecl:
			return x.Tok != token.IMPORT
		case *ast.BinaryExpr:
			if t, ok := turned[x.Op]; ok {
				add(x.OpPos, x.Op.String(), t.String())
			}
		case *ast.IncDecStmt:
			add(x.TokPos, x.Tok.String(), turned[x.Tok].String())
		case *ast.AssignStmt:
			if t, ok := turned[x.Tok]; ok {
				add(x.TokPos, x.Tok.String(), t.String())
			}
		case *ast.UnaryExpr:
			if x.Op == token.NOT {
				add(x.OpPos, "!", "")
			}
		case *ast.Ident:
			switch x.Name {
			case "true":
				add(x.Pos(), "true", "false")
			case "false":
				add(x.Pos(), "false", "true")
			}
		case *ast.BasicLit:
			if v, err := strconv.Atoi(x.Value); err == nil && x.Kind == token.INT {
				add(x.Pos(), x.Value, strconv.Itoa(v+1))
				add(x.Pos(), x.Value, strconv.Itoa(v-1))
			}
		}
		return true
	})
	return ms, nil
}

func lineFilter(s string) (func(int) bool, error) {
	if s == "" {
		return func(int) bool { return true }, nil
	}
	set := map[int]bool{}
	for _, part := range strings.Split(s, ",") {
		a, b, isRange := strings.Cut(part, "-")
		from, err1 := strconv.Atoi(a)
		to, err2 := from, error(nil)
		if isRange {
			to, err2 = strconv.Atoi(b)
		}
		if err1 != nil || err2 != nil || to < from {
			return nil, fmt.Errorf("invalid -lines %q", s)
		}
		for i := from; i <= to; i++ {
			set[i] = true
		}
	}
	return func(l int) bool { return set[l] }, nil
}

// fromSpec reads the list of mutations in the file spec. Each old text must be
// in its file exactly once.
func fromSpec(spec, only string) ([]mutation, error) {
	data, err := os.ReadFile(spec)
	if err != nil {
		return nil, err
	}
	return parseSpec(string(data), only, os.ReadFile)
}

var unescape = strings.NewReplacer("⏎", "\n", "⇥", "\t")

func parseSpec(data, only string, read func(string) ([]byte, error)) ([]mutation, error) {
	var ms []mutation
	var errs []error
	for i, line := range strings.Split(data, "\n") {
		if strings.TrimSpace(line) == "" || strings.HasPrefix(line, "#") {
			continue
		}
		f := strings.Split(line, " | ")
		if len(f) != 4 {
			errs = append(errs, fmt.Errorf("line %d: want name | file | old | new", i+1))
			continue
		}
		name, file, old, repl := f[0], f[1], unescape.Replace(f[2]), unescape.Replace(f[3])
		if !strings.Contains(name, only) {
			continue
		}
		src, err := read(file)
		if err != nil {
			errs = append(errs, fmt.Errorf("line %d: %w", i+1, err))
			continue
		}
		if n := strings.Count(string(src), old); n != 1 {
			errs = append(errs, fmt.Errorf("line %d (%s): the old text is in %s %d times, not once", i+1, name, file, n))
			continue
		}
		start := strings.Index(string(src), old)
		ms = append(ms, mutation{name: name, file: file, start: start, end: start + len(old), repl: repl})
	}
	return ms, errors.Join(errs...)
}
