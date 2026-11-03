package main

import (
	"io"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
	"time"
)

const sample = `package p

import "fmt"

func f(a, b int, ok bool) int {
	if a == b && !ok {
		a++
	}
	b += 2
	fmt.Println(true)
	return a - b
}
`

func names(ms []mutation) []string {
	var ns []string
	for _, m := range ms {
		ns = append(ns, m.name)
	}
	return ns
}

func TestSyntactic(t *testing.T) {
	all := func(int) bool { return true }
	ms, err := syntacticSrc("p.go", []byte(sample), all)
	if err != nil {
		t.Fatal(err)
	}
	want := []string{
		"p.go:6: && -> ||",
		"p.go:6: == -> !=",
		"p.go:6: ! -> ",
		"p.go:7: ++ -> --",
		"p.go:9: += -> -=",
		"p.go:9: 2 -> 3",
		"p.go:9: 2 -> 1",
		"p.go:10: true -> false",
		"p.go:11: - -> +",
	}
	if got := names(ms); !reflect.DeepEqual(got, want) {
		t.Errorf("got  %q\nwant %q", got, want)
	}
	// Each mutation replaces exactly the text it names.
	for _, m := range ms {
		old := strings.Fields(strings.SplitN(m.name, ": ", 2)[1])[0]
		if got := sample[m.start:m.end]; got != old {
			t.Errorf("%s: replaces %q", m.name, got)
		}
	}
}

func TestSyntacticLines(t *testing.T) {
	keep, err := lineFilter("7,9-10")
	if err != nil {
		t.Fatal(err)
	}
	ms, err := syntacticSrc("p.go", []byte(sample), keep)
	if err != nil {
		t.Fatal(err)
	}
	want := []string{"p.go:7: ++ -> --", "p.go:9: += -> -=", "p.go:9: 2 -> 3", "p.go:9: 2 -> 1", "p.go:10: true -> false"}
	if got := names(ms); !reflect.DeepEqual(got, want) {
		t.Errorf("got  %q\nwant %q", got, want)
	}
	for _, bad := range []string{"x", "3-", "5-4", "1,,2"} {
		if _, err := lineFilter(bad); err == nil {
			t.Errorf("lineFilter(%q) succeeded", bad)
		}
	}
}

func TestParseSpec(t *testing.T) {
	files := map[string]string{"a.go": "x := 1\n\tif ok {\n\t\treturn\n\t}\n", "b.go": "y\ny\n"}
	read := func(name string) ([]byte, error) {
		s, ok := files[name]
		if !ok {
			return nil, os.ErrNotExist
		}
		return []byte(s), nil
	}
	spec := "# a comment\n" +
		"\n" +
		"one line | a.go | x := 1 | x := 2\n" +
		"two lines | a.go | ⇥if ok {⏎⇥⇥return | ⇥if false {⏎⇥⇥return\n" +
		"deleted | a.go | ⇥}⏎ | \n"
	ms, err := parseSpec(spec, "", read)
	if err != nil {
		t.Fatal(err)
	}
	want := []mutation{
		{name: "one line", file: "a.go", start: 0, end: 6, repl: "x := 2"},
		{name: "two lines", file: "a.go", start: 7, end: 24, repl: "\tif false {\n\t\treturn"},
		{name: "deleted", file: "a.go", start: 25, end: 28, repl: ""},
	}
	if !reflect.DeepEqual(ms, want) {
		t.Errorf("got  %+v\nwant %+v", ms, want)
	}

	ms, err = parseSpec(spec, "two", read)
	if err != nil || len(ms) != 1 || ms[0].name != "two lines" {
		t.Errorf("-only two: %+v, %v", ms, err)
	}

	// Mistakes are all reported; the good lines are still read.
	bad := "no fields\n" +
		"twice | b.go | y | z\n" +
		"absent | a.go | nothing | z\n" +
		"no file | c.go | y | z\n" +
		"good | a.go | x := 1 | x := 2\n"
	ms, err = parseSpec(bad, "", read)
	if len(ms) != 1 || ms[0].name != "good" {
		t.Errorf("good lines: %+v", ms)
	}
	for _, s := range []string{"line 1: want name | file | old | new", "line 2 (twice): the old text is in b.go 2 times", "line 3 (absent): the old text is in a.go 0 times", "line 4: file does not exist"} {
		if err == nil || !strings.Contains(err.Error(), s) {
			t.Errorf("error %v does not say %q", err, s)
		}
	}
}

// runAll puts every file back, and tells killed from survived from not
// compiled by the test command.
func TestRunAll(t *testing.T) {
	dir := t.TempDir()
	file := filepath.Join(dir, "f.txt")
	if err := os.WriteFile(file, []byte("good\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	// The "tests" fail when the file says bad, and do not build when it says
	// broken.
	script := `case "$(cat "$1")" in bad*) echo "--- FAIL: TestX (0.00s)"; exit 1;; broken*) echo "FAIL p [build failed]"; exit 1;; esac`
	cmd := []string{"sh", "-c", script, "sh", file}
	ms := []mutation{
		{name: "killed", file: file, start: 0, end: 4, repl: "bad"},
		{name: "survived", file: file, start: 0, end: 4, repl: "fine"},
		{name: "not compiled", file: file, start: 0, end: 4, repl: "broken"},
	}
	out := captureStdout(t, func() {
		if code := runAll(ms, cmd, time.Minute); code != 1 {
			t.Errorf("exit code %d, want 1", code)
		}
	})
	want := "killed: killed [TestX]\nSURVIVED: survived\nnot compiled: not compiled (change the mutation: this says nothing about the tests)\nsurvived 1 of 3\n"
	if out != want {
		t.Errorf("got:\n%s\nwant:\n%s", out, want)
	}
	if b, _ := os.ReadFile(file); string(b) != "good\n" {
		t.Errorf("the file was not put back: %q", b)
	}

	// Tests that fail before any mutation make every result meaningless.
	if err := os.WriteFile(file, []byte("bad\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if code := runAll(nil, cmd, time.Minute); code != 2 {
		t.Errorf("failing baseline: exit code %d, want 2", code)
	}
}

func captureStdout(t *testing.T, f func()) string {
	t.Helper()
	r, w, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	old := os.Stdout
	os.Stdout = w
	defer func() { os.Stdout = old }()
	f()
	if err := w.Close(); err != nil {
		t.Fatal(err)
	}
	b, err := io.ReadAll(r)
	if err != nil {
		t.Fatal(err)
	}
	return string(b)
}
