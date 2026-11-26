package tui

// Parity reference generator for the Rust port of the root TUI model.
// Run with: SST_PARITYGEN=1 go test ./pkg/tui -run TestParityGen -count=1
// Writes testdata/parity/tui/*.txt (see the format description in writeHeader).

import (
	"bytes"
	"fmt"
	"hash/fnv"
	"io/fs"
	"math/rand"
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"sort"
	"strings"
	"sync"
	"testing"
	"time"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/muesli/termenv"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/commandbar"
	"github.com/scottdensmore/super-star-trek/pkg/tui/components/scenariomodal"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

// ---------------------------------------------------------------------------
// driver

type pgDriver struct {
	m       Model
	pending []tea.Cmd
	queue   []tea.Msg
	quit    bool
	out     *bytes.Buffer
	step    int
	lastH   string
	ascii   bool
	frames  bool // store frames for this run (subject to budget)
	T       time.Time
	work    string
	names   map[string]bool
	budget  *int
	failed  error
}

func pgFuncName(c tea.Cmd) string {
	return runtime.FuncForPC(reflect.ValueOf(c).Pointer()).Name()
}

func pgIsTimer(name string) bool {
	return strings.Contains(name, "bubbletea.Tick.func") ||
		strings.Contains(name, "bubbletea.Every.func") ||
		strings.Contains(name, "cursor.(*Model).BlinkCmd.func")
}

func (d *pgDriver) expand(c tea.Cmd) {
	if c == nil {
		return
	}
	name := pgFuncName(c)
	timer := pgIsTimer(name)
	d.names[fmt.Sprintf("%s timer=%v", name, timer)] = true
	if timer {
		d.pending = append(d.pending, c)
		return
	}
	d.expandMsg(c())
}

func (d *pgDriver) expandMsg(msg tea.Msg) {
	if msg == nil {
		return
	}
	if b, ok := msg.(tea.BatchMsg); ok {
		for _, c := range b {
			d.expand(c)
		}
		return
	}
	rt := reflect.TypeOf(msg)
	if rt.String() == "tea.sequenceMsg" {
		v := reflect.ValueOf(msg)
		for i := 0; i < v.Len(); i++ {
			c, _ := v.Index(i).Interface().(tea.Cmd)
			d.expand(c)
		}
		return
	}
	d.queue = append(d.queue, msg)
}

func (d *pgDriver) process() {
	for len(d.queue) > 0 {
		msg := d.queue[0]
		d.queue = d.queue[1:]
		if _, ok := msg.(tea.QuitMsg); ok {
			d.quit = true
			continue
		}
		var cmd tea.Cmd
		d.m, cmd = d.m.UpdateModel(msg)
		if ls, ok := msg.(scenariomodal.MsgLaunchScenario); ok {
			sc, ok2 := engine.GetScenario(ls.ScenarioID)
			if ok2 && sc.Build != nil && d.m.Game != nil {
				rebuilt := sc.Build(d.T.UnixNano())
				if rebuilt.Enterprise.Condition != d.m.Game.Enterprise.Condition {
					d.failed = fmt.Errorf("scenario %s: random-seed condition %v != fixed-seed condition %v", ls.ScenarioID, d.m.Game.Enterprise.Condition, rebuilt.Enterprise.Condition)
				}
				if rebuilt.Rules.AnimSpeed != d.m.Game.Rules.AnimSpeed {
					d.failed = fmt.Errorf("scenario %s anim speed differs", ls.ScenarioID)
				}
				d.m.Game = rebuilt
			}
		}
		d.expand(cmd)
	}
}

func (d *pgDriver) touchFiles() {
	var files []string
	_ = filepath.WalkDir(d.work, func(p string, e fs.DirEntry, err error) error {
		if err == nil && e.Type().IsRegular() {
			files = append(files, p)
		}
		return nil
	})
	sort.Strings(files)
	for i, p := range files {
		t := d.T.Add(-time.Duration(i) * time.Minute)
		_ = os.Chtimes(p, t, t)
	}
}

func (d *pgDriver) record(line string) {
	fmt.Fprintf(d.out, "step %d %s\n", d.step, line)
	d.touchFiles()
	view := d.m.View()
	h := fnv.New64a()
	_, _ = h.Write([]byte(view))
	hs := fmt.Sprintf("%016x", h.Sum64())
	q := 0
	if d.quit {
		q = 1
	}
	fmt.Fprintf(d.out, "= %s pend=%d quit=%d\n", hs, len(d.pending), q)
	if d.frames && hs != d.lastH && *d.budget >= len(view) {
		*d.budget -= len(view)
		fmt.Fprintf(d.out, "frame %d\n%s\n", len(view), view)
	}
	d.lastH = hs
	d.step++
}

func (d *pgDriver) finalFrame() {
	view := d.m.View()
	fmt.Fprintf(d.out, "frame %d\n%s\n", len(view), view)
}

// ops ---------------------------------------------------------------------

func (d *pgDriver) send(line string, msg tea.Msg) {
	d.queue = append(d.queue, msg)
	d.process()
	d.record(line)
}

func (d *pgDriver) size(w, h int) {
	d.send(fmt.Sprintf("size %d %d", w, h), tea.WindowSizeMsg{Width: w, Height: h})
}

func (d *pgDriver) rawKey(k tea.KeyMsg) {
	alt := 0
	if k.Alt {
		alt = 1
	}
	d.send(fmt.Sprintf("key %d %q %d", int(k.Type), string(k.Runes), alt), k)
}

var pgNamedKeys = map[string]tea.KeyType{
	"enter": tea.KeyEnter, "esc": tea.KeyEsc, "up": tea.KeyUp, "down": tea.KeyDown,
	"left": tea.KeyLeft, "right": tea.KeyRight, "tab": tea.KeyTab, "shift+tab": tea.KeyShiftTab,
	"backspace": tea.KeyBackspace, "delete": tea.KeyDelete, "home": tea.KeyHome, "end": tea.KeyEnd,
	"pgup": tea.KeyPgUp, "pgdown": tea.KeyPgDown, "f1": tea.KeyF1, "f2": tea.KeyF2, "f3": tea.KeyF3,
	"ctrl+p": tea.KeyCtrlP, "ctrl+o": tea.KeyCtrlO, "ctrl+g": tea.KeyCtrlG, "ctrl+d": tea.KeyCtrlD,
	"ctrl+h": tea.KeyCtrlH, "ctrl+s": tea.KeyCtrlS, "ctrl+t": tea.KeyCtrlT, "ctrl+c": tea.KeyCtrlC,
	"ctrl+u": tea.KeyCtrlU, "ctrl+a": tea.KeyCtrlA, "ctrl+e": tea.KeyCtrlE, "ctrl+k": tea.KeyCtrlK,
	"ctrl+w": tea.KeyCtrlW,
}

// key sends a named key ("enter", "ctrl+p", ...) or a single rune.
func (d *pgDriver) key(name string) {
	if kt, ok := pgNamedKeys[name]; ok {
		d.rawKey(tea.KeyMsg{Type: kt})
		return
	}
	if name == " " || name == "space" {
		d.rawKey(tea.KeyMsg{Type: tea.KeySpace, Runes: []rune{' '}})
		return
	}
	if strings.HasPrefix(name, "alt+") {
		d.rawKey(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune(name[4:]), Alt: true})
		return
	}
	d.rawKey(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune(name)})
}

func (d *pgDriver) keys(names ...string) {
	for _, n := range names {
		d.key(n)
	}
}

func (d *pgDriver) typ(s string) {
	for _, r := range s {
		d.key(string(r))
	}
}

func (d *pgDriver) cmdline(s string) {
	d.typ(s)
	d.key("enter")
}

func (d *pgDriver) submit(s string) {
	d.send(fmt.Sprintf("submit %q", s), commandbar.CommandSubmittedMsg{Text: s})
}

func (d *pgDriver) mouse(x, y int, b tea.MouseButton, a tea.MouseAction) {
	d.send(fmt.Sprintf("mouse %d %d %d %d", x, y, int(b), int(a)), tea.MouseMsg{X: x, Y: y, Button: b, Action: a})
}

func (d *pgDriver) click(x, y int) {
	d.m.LastClickTime = time.Time{}
	d.queue = append(d.queue, tea.MouseMsg{X: x, Y: y, Button: tea.MouseButtonLeft, Action: tea.MouseActionPress})
	d.process()
	d.record(fmt.Sprintf("click %d %d", x, y))
}

func (d *pgDriver) dblclick(x, y int) {
	d.m.LastClickTime = time.Time{}
	d.queue = append(d.queue, tea.MouseMsg{X: x, Y: y, Button: tea.MouseButtonLeft, Action: tea.MouseActionPress})
	d.process()
	d.queue = append(d.queue, tea.MouseMsg{X: x, Y: y, Button: tea.MouseButtonLeft, Action: tea.MouseActionPress})
	d.process()
	d.record(fmt.Sprintf("dblclick %d %d", x, y))
}

func (d *pgDriver) gameover(reason engine.GameOverReason, score float64) {
	d.send(fmt.Sprintf("gameover %d %g", int(reason), score), engine.EventGameOver{Reason: reason, Score: score})
}

func (d *pgDriver) tick() {
	cur := d.pending
	d.pending = nil
	res := make([]tea.Msg, len(cur))
	var wg sync.WaitGroup
	for i, c := range cur {
		wg.Add(1)
		go func(i int, c tea.Cmd) {
			defer wg.Done()
			res[i] = c()
		}(i, c)
	}
	wg.Wait()
	for _, r := range res {
		d.expandMsg(r)
	}
	d.process()
	d.record("tick")
}

func (d *pgDriver) ticks(n int) {
	for i := 0; i < n; i++ {
		d.tick()
	}
}

// tickUntilIdle fires ticks until no animation is running (max n).
func (d *pgDriver) tickAnim(n int) {
	for i := 0; i < n && d.m.activeAnim != nil; i++ {
		d.tick()
	}
}

// geometry helpers ------------------------------------------------------------

func pgSectorXY(c engine.Coord) (int, int) {
	return 3 + 4*(c[1]-1) + 1, c[0] + 2
}

func (d *pgDriver) chartXY(c engine.Coord) (int, int) {
	w := d.m.GalacticChart.Width()
	if w <= 0 {
		w = 64
	}
	h := d.m.GalacticChart.Height()
	if h <= 0 {
		h = 18
	}
	sx := (d.m.Width - w) / 2
	sy := (d.m.Height - h) / 2
	return sx + 7 + 7*(c[1]-1) + 2, sy + c[0] + 3
}

func (d *pgDriver) game() *engine.GameState { return d.m.Game }

func (d *pgDriver) firstHostile() (engine.Coord, bool) {
	g := d.game()
	if g == nil {
		return engine.Coord{}, false
	}
	for _, k := range g.CurrentQuad.Klingons {
		if k != nil && !k.IsCloaked {
			return k.Sector, true
		}
	}
	for _, e := range g.CurrentQuad.Enemies {
		if e != nil && !e.IsCloaked {
			return e.Sector, true
		}
	}
	return engine.Coord{}, false
}

// anyEnemy returns the first enemy or Klingon sector, cloaked or not.
func (d *pgDriver) anyEnemy() (engine.Coord, bool) {
	g := d.game()
	if g == nil {
		return engine.Coord{}, false
	}
	for _, e := range g.CurrentQuad.Enemies {
		if e != nil {
			return e.Sector, true
		}
	}
	for _, k := range g.CurrentQuad.Klingons {
		if k != nil {
			return k.Sector, true
		}
	}
	return engine.Coord{}, false
}

func (d *pgDriver) emptyNear() (engine.Coord, bool) {
	g := d.game()
	if g == nil {
		return engine.Coord{}, false
	}
	s := g.Enterprise.Sector
	for _, dd := range [][2]int{{0, 1}, {1, 0}, {0, -1}, {-1, 0}, {1, 1}, {-1, -1}, {0, 2}, {2, 0}} {
		r, c := s[0]+dd[0], s[1]+dd[1]
		if r >= 1 && r <= 8 && c >= 1 && c <= 8 && g.CurrentQuad.Grid[r][c] == engine.EntityEmpty {
			return engine.Coord{r, c}, true
		}
	}
	return engine.Coord{}, false
}

func (d *pgDriver) starbase() (engine.Coord, bool) {
	g := d.game()
	if g == nil || g.CurrentQuad.Starbase == nil {
		return engine.Coord{}, false
	}
	return *g.CurrentQuad.Starbase, true
}

// emptyAdjacentTo returns an empty sector adjacent to c.
func (d *pgDriver) emptyAdjacentTo(c engine.Coord) (engine.Coord, bool) {
	g := d.game()
	for dr := -1; dr <= 1; dr++ {
		for dc := -1; dc <= 1; dc++ {
			r, cc := c[0]+dr, c[1]+dc
			if (dr != 0 || dc != 0) && r >= 1 && r <= 8 && cc >= 1 && cc <= 8 && g.CurrentQuad.Grid[r][cc] == engine.EntityEmpty {
				return engine.Coord{r, cc}, true
			}
		}
	}
	return engine.Coord{}, false
}

// ---------------------------------------------------------------------------
// sessions

type pgSession struct {
	name   string
	setup  string
	frames bool
	script func(d *pgDriver)
}

func pgParseSetup(s string) map[string]string {
	kv := map[string]string{}
	for _, f := range strings.Fields(s) {
		p := strings.SplitN(f, "=", 2)
		kv[p[0]] = p[1]
	}
	return kv
}

func pgAtoi(s string) int {
	var n int
	fmt.Sscanf(s, "%d", &n)
	return n
}

func pgRules(kv map[string]string) engine.GameRules {
	prof := kv["profile"]
	if prof == "" {
		prof = "normal"
	}
	rules := engine.DefaultRulesForProfile(engine.DifficultyProfile(prof))
	if kv["adv"] == "1" {
		rules.Adversaries = true
		rules.Profile = engine.ProfileCustom
	}
	if kv["anom"] == "1" {
		rules.SpatialAnomalies = true
		rules.Profile = engine.ProfileCustom
	}
	if kv["cloak"] == "1" {
		rules.KlingonCloak = true
		rules.Profile = engine.ProfileCustom
	}
	if a, ok := kv["anim"]; ok {
		rules.AnimSpeed = pgAtoi(a)
	}
	return rules
}

func pgBuildGame(kv map[string]string) *engine.GameState {
	seed := int64(pgAtoi(kv["seed"]))
	g := engine.NewGameWithOptions(seed, engine.SkillGood, engine.LengthMedium, pgRules(kv))
	g.PopulateQuadrant(g.Enterprise.Quad, g.Enterprise.Sector)
	return g
}

func pgBuildModel(kv map[string]string) Model {
	mode, _ := theme.ParseColorMode(kv["mode"])
	th := theme.GetTheme(kv["theme"]).WithColorMode(mode)
	seed := int64(pgAtoi(kv["seed"]))
	var m Model
	switch kv["kind"] {
	case "game":
		m = NewModel(pgBuildGame(kv), th)
	case "scenario":
		sc, ok := engine.GetScenario(engine.ScenarioID(kv["id"]))
		if !ok {
			panic("unknown scenario " + kv["id"])
		}
		m = NewModel(sc.Build(seed), th)
	case "tour":
		m = NewModelWithTour(engine.NewTour(seed), th)
	case "newgame":
		m = NewModel(engine.NewGame(seed, engine.SkillGood, engine.LengthMedium), th)
	case "nil":
		m = NewModel(nil, th)
	default:
		panic("bad kind " + kv["kind"])
	}
	if s, ok := kv["sound"]; ok {
		m.SetSoundEnabled(s == "1")
	}
	if v, ok := kv["volume"]; ok && m.AudioPlayer != nil {
		m.AudioPlayer.SetVolume(pgAtoi(v))
	}
	return m
}

// pgFindSeed returns the first seed >= from whose game satisfies pred.
func pgFindSeed(from int64, extra string, pred func(g *engine.GameState) bool) int64 {
	for s := from; s < from+5000; s++ {
		kv := pgParseSetup(fmt.Sprintf("seed=%d %s", s, extra))
		if pred(pgBuildGame(kv)) {
			return s
		}
	}
	panic("no seed found")
}

func hasKlingons(g *engine.GameState) bool { return len(g.CurrentQuad.Klingons) > 0 }
func hasStarbase(g *engine.GameState) bool { return g.CurrentQuad.Starbase != nil }

const pgStd = "theme=modern mode=dark sound=1 volume=80"

func pgCombatScript(d *pgDriver) {
	d.size(80, 24)
	d.cmdline("srscan")
	d.cmdline("status")
	if c, ok := d.firstHostile(); ok {
		d.cmdline(fmt.Sprintf("tor %d %d", c[0], c[1]))
		d.tickAnim(30)
	}
	d.ticks(2)
	d.cmdline("pha 300")
	d.tickAnim(30)
	d.cmdline("she 400")
	d.ticks(3)
	if c, ok := d.firstHostile(); ok {
		d.submit(fmt.Sprintf("tor %d %d", c[0], c[1]))
		d.tickAnim(30)
	}
	d.submit("tor 1.57")
	d.tickAnim(30)
	d.submit("tor c 3")
	d.tickAnim(30)
	d.submit("pha 500")
	d.tickAnim(30)
	d.ticks(4)
}

func pgSessions() map[string][]pgSession {
	files := map[string][]pgSession{}

	// ---------------- games.txt
	combatSeed := pgFindSeed(100, "profile=normal", hasKlingons)
	hardSeed := pgFindSeed(300, "profile=hardcore", hasKlingons)
	advSeed := pgFindSeed(500, "profile=normal adv=1 anom=1 cloak=1", func(g *engine.GameState) bool {
		return len(g.CurrentQuad.Enemies) > 0
	})
	dockSeed := pgFindSeed(700, "profile=casual", hasStarbase)
	nightSeed := pgFindSeed(900, "profile=nightmare", hasKlingons)

	files["games"] = []pgSession{
		{name: "game_basic_s42", setup: "kind=game seed=42 profile=normal adv=0 anom=0 cloak=0 " + pgStd, frames: true, script: func(d *pgDriver) {
			d.size(80, 24)
			d.cmdline("srscan")
			d.cmdline("lrscan")
			d.cmdline("status")
			d.cmdline("she 500")
			d.cmdline("she -200")
			d.cmdline("nav 1.0 1")
			d.cmdline("nav q 4 4")
			d.cmdline("nav s 2 2")
			d.cmdline("anim")
			d.cmdline("anim fast")
			d.cmdline("anim 9")
			d.cmdline("anim normal extra")
			d.cmdline("color light")
			d.cmdline("color bogus")
			d.cmdline("color")
			d.cmdline("theme mode dark")
			d.cmdline("theme mode")
			d.cmdline("theme mode neon")
			d.cmdline("theme lcars")
			d.cmdline("theme")
			d.cmdline("tour")
			d.cmdline("orders")
			d.cmdline("xyzzy")
			d.cmdline("nav")
			d.cmdline("tor abc")
			d.cmdline("pha")
			d.cmdline("thaw NOPE.TRK")
			d.typ("abc")
			d.key("esc")
			d.key("ctrl+s")
			d.key("m")
			d.typ("doc")
			d.key("enter")
			d.key("up")
			d.key("down")
			d.cmdline("help she")
			d.key("esc")
		}},
		{name: "game_combat_normal", setup: fmt.Sprintf("kind=game seed=%d profile=normal adv=0 anom=0 cloak=0 %s", combatSeed, pgStd), frames: true, script: pgCombatScript},
		{name: "game_combat_hardcore_fast", setup: fmt.Sprintf("kind=game seed=%d profile=hardcore adv=0 anom=0 cloak=0 anim=1 theme=lcars mode=dark sound=0 volume=50", hardSeed), frames: true, script: pgCombatScript},
		{name: "game_combat_nightmare_cine", setup: fmt.Sprintf("kind=game seed=%d profile=nightmare adv=0 anom=0 cloak=0 anim=3 theme=crt mode=light sound=1 volume=30", nightSeed), frames: true, script: pgCombatScript},
		{name: "game_combat_animoff", setup: fmt.Sprintf("kind=game seed=%d profile=normal adv=0 anom=0 cloak=0 anim=0 %s", combatSeed, pgStd), frames: false, script: pgCombatScript},
		{name: "game_adversaries", setup: fmt.Sprintf("kind=game seed=%d profile=normal adv=1 anom=1 cloak=1 %s", advSeed, pgStd), frames: true, script: func(d *pgDriver) {
			d.size(100, 30)
			pgCombatScript(d)
			d.key("t")
			d.keys("tab", "tab", "left", "enter")
			d.tickAnim(30)
			d.key("t")
			d.keys("p", "2", "0", "0", "enter")
			d.tickAnim(30)
			for i := 0; i < 6; i++ {
				d.cmdline("lrscan")
				d.cmdline(fmt.Sprintf("nav q %d %d", 1+(i*3)%8, 1+(i*5)%8))
				d.ticks(1)
				if c, ok := d.firstHostile(); ok {
					d.cmdline(fmt.Sprintf("tor %d %d", c[0], c[1]))
					d.tickAnim(30)
				}
			}
		}},
		{name: "game_dock_casual", setup: fmt.Sprintf("kind=game seed=%d profile=casual adv=0 anom=0 cloak=0 %s", dockSeed, pgStd), frames: true, script: func(d *pgDriver) {
			d.size(80, 24)
			d.cmdline("she 300")
			if sb, ok := d.starbase(); ok {
				x, y := pgSectorXY(sb)
				d.dblclick(x, y) // probably not adjacent -> error
				if e, ok := d.emptyAdjacentTo(sb); ok {
					d.cmdline(fmt.Sprintf("nav s %d %d", e[0], e[1]))
					d.dblclick(x, y)
					d.cmdline("doc")
				}
				d.click(x, y)
			}
			if e, ok := d.emptyNear(); ok {
				x, y := pgSectorXY(e)
				d.click(x, y)
				d.dblclick(x, y)
			}
			d.click(1, 1)
			d.click(5, 30)
			d.mouse(10, 5, tea.MouseButtonLeft, tea.MouseActionRelease)
			d.mouse(10, 5, tea.MouseButtonRight, tea.MouseActionPress)
			d.mouse(10, 5, tea.MouseButtonWheelUp, tea.MouseActionPress)
			d.cmdline("dam")
			d.key("esc")
		}},
		{name: "game_click_targets", setup: fmt.Sprintf("kind=game seed=%d profile=normal adv=1 anom=0 cloak=0 %s", advSeed, pgStd), frames: true, script: func(d *pgDriver) {
			d.size(80, 24)
			if c, ok := d.firstHostile(); ok {
				x, y := pgSectorXY(c)
				d.click(x, y)
				d.keys("tab", "enter")
				d.tickAnim(30)
				d.click(x, y)
				d.key("esc")
				d.dblclick(x, y)
			}
			for r := 1; r <= 8; r += 3 {
				for c := 1; c <= 8; c += 3 {
					x, y := pgSectorXY(engine.Coord{r, c})
					d.click(x, y)
					if d.m.ActiveModal != ModalNone {
						d.key("esc")
					}
				}
			}
		}},
		{name: "game_sizes", setup: "kind=game seed=7 profile=normal adv=0 anom=0 cloak=0 theme=lcars mode=light sound=1 volume=80", frames: true, script: func(d *pgDriver) {
			d.size(60, 20)
			d.cmdline("srscan")
			d.size(80, 24)
			d.size(100, 30)
			d.size(114, 30)
			d.size(115, 30)
			d.size(120, 40)
			d.cmdline("lrscan")
			d.cmdline("nav q 2 7")
			d.key("c")
			d.size(120, 40)
			d.key("esc")
			d.key("ctrl+p")
			d.size(60, 20)
			d.size(0, 0)
			d.size(150, 50)
			d.key("esc")
			d.key("ctrl+t")
			d.key("f2")
		}},
		{name: "game_newgame_12345", setup: "kind=newgame seed=12345 " + pgStd, frames: true, script: func(d *pgDriver) {
			d.size(80, 24)
			d.cmdline("lrscan")
			d.size(120, 40)
			d.key("t")
			d.key("d")
			d.key("esc")
		}},
		{name: "nil_model", setup: "kind=nil theme=crt mode=dark", frames: true, script: func(d *pgDriver) {
			d.size(80, 24)
			d.key("t")
			d.key("c")
			d.key("esc")
			d.key("d")
			d.key("esc")
			d.cmdline("lrscan")
			d.cmdline("nav 1 1")
			d.cmdline("anim")
			d.cmdline("anim fast")
			d.cmdline("tour")
			d.key("h")
			d.key("esc")
			d.size(120, 40)
			d.key("o")
			d.key("esc")
			d.click(5, 4)
			d.dblclick(5, 4)
		}},
	}

	// ---------------- modals.txt
	files["modals"] = []pgSession{
		{name: "themes_cycle", setup: "kind=game seed=11 profile=normal adv=0 anom=0 cloak=0 theme=modern mode=dark sound=1 volume=80", frames: true, script: func(d *pgDriver) {
			d.size(80, 24)
			for i := 0; i < 3; i++ {
				for j := 0; j < 4; j++ {
					d.key("ctrl+t")
				}
				d.key("f2")
			}
			d.cmdline("theme crt")
			d.cmdline("color dark")
			d.cmdline("theme mode light")
			d.cmdline("theme mode auto")
			d.cmdline("theme lcars")
			d.cmdline("theme nope")
			d.key("c")
			d.key("f2")
			d.key("ctrl+t")
			d.key("esc")
		}},
		{name: "options_modal", setup: "kind=game seed=12 profile=normal adv=0 anom=0 cloak=0 theme=modern mode=dark sound=1 volume=80", frames: true, script: func(d *pgDriver) {
			d.size(80, 24)
			d.key("o")
			for i := 0; i < 10; i++ {
				d.key("down")
				d.key("right")
			}
			d.keys("up", "left", "h", "l", "j", "k", " ")
			d.key("f2")
			d.key("ctrl+t")
			d.click(10, 10)
			d.key("enter")
			d.cmdline("options")
			d.keys("down", "down", "right", "right", "esc")
			d.cmdline("settings")
			d.key("q")
			d.cmdline("opts")
			for i := 0; i < 12; i++ {
				d.key("down")
			}
			d.keys("left", "left", "enter")
			d.size(100, 30)
		}},
		{name: "palette", setup: "kind=game seed=13 profile=normal adv=0 anom=0 cloak=0 theme=lcars mode=dark sound=1 volume=80", frames: true, script: func(d *pgDriver) {
			d.size(80, 24)
			d.key("ctrl+p")
			d.keys("down", "down", "pgdown", "pgup", "end", "home", "up")
			d.typ("nav")
			d.key("enter")
			d.key("esc")
			d.key("/")
			d.typ("lr")
			d.key("backspace")
			d.typ("r")
			d.key("enter")
			d.key("ctrl+p")
			d.typ("zzzz")
			d.key("enter")
			d.key("delete")
			d.key("backspace")
			d.key(" ")
			d.key("esc")
			d.key("ctrl+p")
			d.typ("chart")
			d.key("enter")
			d.key("esc")
			d.key("ctrl+p")
			d.typ("help")
			d.key("enter")
			d.key("esc")
		}},
		{name: "targetlock", setup: fmt.Sprintf("kind=game seed=%d profile=normal adv=0 anom=0 cloak=0 %s", combatSeed, pgStd), frames: true, script: func(d *pgDriver) {
			d.size(80, 24)
			d.key("T")
			d.keys("tab", "right", "shift+tab", "left")
			d.keys("p", "1", "x", "5", ".", "0", "backspace", "enter")
			d.tickAnim(30)
			d.cmdline("target")
			d.keys("p", "enter")
			d.keys("p", "p", "enter")
			d.tickAnim(30)
			d.key("t")
			d.key("ctrl+t")
			d.key("f2")
			d.key("esc")
			d.key("t")
			d.keys("p", "9", "9", "9", "9", "enter")
			d.tickAnim(30)
			for i := 0; i < 12; i++ {
				d.key("t")
				d.key("enter")
				d.tickAnim(30)
			}
		}},
		{name: "chart", setup: "kind=game seed=14 profile=normal adv=0 anom=0 cloak=0 theme=crt mode=dark sound=1 volume=80", frames: true, script: func(d *pgDriver) {
			d.size(80, 24)
			d.cmdline("lrscan")
			d.key("c")
			d.keys("up", "k", "left", "h", "down", "j", "right", "l", "K", "J", "H", "L")
			d.key("?")
			d.key("x")
			d.key("?")
			x, y := d.chartXY(engine.Coord{2, 3})
			d.click(x, y)
			d.key("?")
			d.click(x, y)
			d.click(x, y)
			d.click(0, 0)
			d.dblclick(x, y)
			d.tick()
			d.key("ctrl+g")
			d.keys("down", "down", "enter")
			d.ticks(2)
			d.cmdline("chart")
			d.key("esc")
			d.cmdline("map")
			d.key("q")
			d.key("esc")
			d.size(120, 40)
			d.key("ctrl+g")
			x, y = d.chartXY(engine.Coord{7, 7})
			d.dblclick(x, y)
			d.ticks(2)
		}},
		{name: "damage_manual", setup: fmt.Sprintf("kind=game seed=%d profile=hardcore adv=0 anom=0 cloak=0 %s", hardSeed, pgStd), frames: true, script: func(d *pgDriver) {
			d.size(80, 24)
			d.key("d")
			d.key("x")
			d.key("enter")
			d.key("ctrl+d")
			d.key("q")
			d.cmdline("damage")
			d.key(" ")
			d.key("f1")
			for i := 0; i < 4; i++ {
				d.key("down")
			}
			d.keys("enter", "down", "down", "pgdown", " ", "pgup", "end", "home", "G", "g", "b", "left", "tab", "j", "k", "right", "esc", "tab", "esc")
			d.key("?")
			d.key("esc")
			for _, topic := range []string{"help nav", "help tor", "help pha", "help she", "help doc", "help chart", "help saves", "help options", "help bogus", "man nav", "manual tor", "docs", "codex pha", "guide warp", "help", "commands", "man 3", "help 5 extra"} {
				d.cmdline(topic)
				d.key("esc")
				if d.m.ActiveModal != ModalNone {
					d.key("esc")
				}
			}
			d.submit("help")
			d.key("esc")
			// damage via combat to see damaged schematic
			pgCombatScript(d)
			d.key("d")
			d.key("esc")
			d.key("ctrl+h")
			d.key("tab")
			d.key("esc")
		}},
		{name: "hall_of_fame", setup: "kind=game seed=15 profile=normal adv=0 anom=0 cloak=0 theme=modern mode=light sound=1 volume=80", frames: true, script: func(d *pgDriver) {
			d.size(80, 24)
			d.key("h")
			d.keys("tab", "left", "right", "2", "1", "shift+tab", "q")
			d.cmdline("score")
			d.key("esc")
			d.cmdline("hof")
			d.key(" ")
			d.gameover(engine.GameOverWon, 1234)
			d.typ("Kirk")
			d.key("backspace")
			d.typ("k J")
			d.key("enter")
			d.key("tab")
			d.key("esc")
			d.gameover(engine.GameOverDestroyed, 0)
			d.key("enter")
			d.key("enter")
			d.gameover(engine.GameOverTime, 0)
			d.key("esc")
			d.gameover(engine.GameOverEnergy, 0)
			d.typ("Picard")
			d.key("enter")
			d.key("enter")
			d.gameover(engine.GameOverStranded, 0)
			d.key("esc")
			d.gameover(engine.GameOverLost, 0)
			d.key("esc")
			d.key("h")
			d.key("2")
			d.key("esc")
		}},
		{name: "save_browser", setup: "kind=game seed=16 profile=normal adv=0 anom=0 cloak=0 saves=3 theme=lcars mode=dark sound=1 volume=80", frames: true, script: func(d *pgDriver) {
			d.size(80, 24)
			d.key("ctrl+o")
			d.keys("down", "down", "down", "up", "j", "k")
			d.key("enter")
			d.key("ctrl+o")
			d.keys("d", "n", "d", "esc")
			d.key("ctrl+o")
			d.key("delete")
			d.key("y")
			d.key("esc")
			d.cmdline("saves")
			d.key("enter")
			d.cmdline("thaw MISSION1.TRK")
			d.cmdline("thaw missing.TRK")
			d.cmdline("thaw")
			d.key("esc")
			d.cmdline("lrscan")
			d.size(120, 40)
			d.key("ctrl+o")
			d.key("enter")
		}},
		{name: "save_browser_empty", setup: "kind=game seed=17 profile=normal adv=0 anom=0 cloak=0 theme=crt mode=light sound=0 volume=10", frames: true, script: func(d *pgDriver) {
			d.size(80, 24)
			d.key("ctrl+o")
			d.key("enter")
			d.key("esc")
		}},
	}

	// ---------------- scenarios.txt
	var scen []pgSession
	for i, sc := range engine.ListScenarios() {
		id := string(sc.ID)
		scen = append(scen, pgSession{name: "scenario_" + id, setup: fmt.Sprintf("kind=scenario id=%s seed=%d %s", id, 4242+i, pgStd), frames: true, script: func(d *pgDriver) {
			d.size(80, 24)
			d.cmdline("srscan")
			d.cmdline("status")
			d.cmdline("lrscan")
			d.ticks(3)
			pgCombatScript(d)
			d.key("t")
			d.key("enter")
			d.tickAnim(30)
			d.size(120, 40)
			d.ticks(2)
			d.cmdline("nav 3.14 1")
			d.ticks(2)
		}})
	}
	scen = append(scen, pgSession{name: "scenario_modal", setup: "kind=game seed=18 profile=normal adv=0 anom=0 cloak=0 " + pgStd, frames: true, script: func(d *pgDriver) {
		d.size(80, 24)
		d.cmdline("scenarios")
		d.keys("down", "down", "down", "up", "j", "k")
		d.key("esc")
		d.cmdline("scen")
		d.key("q")
		d.cmdline("scenarios")
		d.key("enter")
		d.ticks(3)
		d.cmdline("scenarios")
		d.keys("down", "enter")
		d.ticks(2)
		d.cmdline("scenarios")
		d.keys("down", "down", "enter")
		d.ticks(2)
		pgCombatScript(d)
	}})
	files["scenarios"] = scen

	// ---------------- tour.txt
	tourScript := func(d *pgDriver) {
		d.size(80, 24)
		d.cmdline("tour")
		d.cmdline("orders")
		for iter := 0; iter < 40 && d.m.Tour != nil && !d.m.Tour.InDrydock && d.m.Tour.Active; iter++ {
			if c, ok := d.anyEnemy(); ok && iter < 4 {
				if iter%2 == 1 {
					d.cmdline("pha 600")
				} else {
					d.cmdline(fmt.Sprintf("tor %d %d", c[0], c[1]))
				}
				d.tickAnim(30)
				if iter%5 == 4 {
					d.cmdline("she 300")
				}
			} else {
				d.cmdline("lrscan")
				d.cmdline(fmt.Sprintf("nav q %d %d", 1+(iter*3)%8, 1+(iter*5)%8))
			}
		}
		for round := 0; round < 3 && d.m.Tour != nil && d.m.Tour.InDrydock; round++ {
			d.keys("down", "enter", "down", "enter", "up", "k", "j", "enter", "enter", "enter")
			d.key("f2")
			d.key("ctrl+t")
			d.click(10, 10)
			d.key("D")
			d.cmdline("orders")
			d.size(120, 40)
			// next sector: fight a little, then warp until cleared again
			for iter := 0; iter < 12 && d.m.Tour != nil && !d.m.Tour.InDrydock && d.m.Tour.Active; iter++ {
				if c, ok := d.anyEnemy(); ok && iter < 2 {
					d.cmdline(fmt.Sprintf("tor %d %d", c[0], c[1]))
					d.tickAnim(30)
				} else {
					d.cmdline(fmt.Sprintf("nav q %d %d", 1+(iter*5+round)%8, 1+(iter*3+round)%8))
				}
			}
			d.size(80, 24)
		}
		d.gameover(engine.GameOverDestroyed, 0)
		d.key("esc")
		d.cmdline("tour")
		d.key("h")
		d.key("tab")
		d.key("esc")
	}
	files["tour"] = []pgSession{
		{name: "tour_s1", setup: "kind=tour seed=1 " + pgStd, frames: true, script: tourScript},
		{name: "tour_s77", setup: "kind=tour seed=77 theme=lcars mode=light sound=1 volume=60", frames: true, script: tourScript},
		{name: "tour_win", setup: "kind=tour seed=5 theme=crt mode=dark sound=1 volume=80", frames: true, script: func(d *pgDriver) {
			d.size(100, 30)
			d.gameover(engine.GameOverWon, 999)
			d.typ("Sulu")
			d.key("enter")
			if d.m.Tour != nil && d.m.Tour.InDrydock {
				d.keys("enter", "space")
			}
			d.cmdline("orders")
			for i := 0; i < 4; i++ {
				d.gameover(engine.GameOverWon, 100)
				d.key("esc")
				if d.m.Tour != nil && d.m.Tour.InDrydock {
					d.keys("down", "enter", "d")
				}
				d.cmdline("tour")
			}
		}},
	}

	// ---------------- mash.txt
	var mash []pgSession
	mashSetups := []string{
		fmt.Sprintf("kind=game seed=%d profile=normal adv=1 anom=1 cloak=1 %s", advSeed, pgStd),
		fmt.Sprintf("kind=game seed=%d profile=hardcore adv=0 anom=1 cloak=0 anim=1 saves=2 theme=lcars mode=light sound=1 volume=50", hardSeed),
		"kind=tour seed=3 theme=crt mode=dark sound=0 volume=70",
	}
	for i, st := range mashSetups {
		seed := int64(9000 + i)
		mash = append(mash, pgSession{name: fmt.Sprintf("mash_%d", i), setup: st, frames: false, script: func(d *pgDriver) {
			pgMash(d, rand.New(rand.NewSource(seed)), 400)
		}})
	}
	files["mash"] = mash
	return files
}

func pgMash(d *pgDriver, r *rand.Rand, n int) {
	sizes := [][2]int{{80, 24}, {100, 30}, {120, 40}, {60, 20}}
	named := []string{"enter", "esc", "up", "down", "left", "right", "tab", "shift+tab", "backspace", "delete",
		"home", "end", "pgup", "pgdown", "f1", "f2", "ctrl+p", "ctrl+o", "ctrl+g", "ctrl+d", "ctrl+h", "ctrl+s",
		"ctrl+t", "ctrl+u", "space"}
	letters := []string{"t", "T", "c", "C", "d", "D", "h", "H", "o", "O", "m", "M", "?", "/", "j", "k", "l", "p", "q", "1", "2", "3", "5", "0", "."}
	cmds := []string{"srscan", "lrscan", "status", "she 300", "she -100", "pha 200", "tor 4 4", "tor 1.0", "nav q 3 3",
		"nav s 4 5", "nav 2.0 0.5", "doc", "dam", "chart", "saves", "score", "options", "tour", "help", "help nav",
		"theme", "theme crt", "color light", "color dark", "theme mode auto", "anim", "anim off", "anim fast", "scenarios", "target",
		"thaw MISSION1.TRK", "man 4", "orders"}
	d.size(80, 24)
	for i := 0; i < n; i++ {
		switch p := r.Intn(100); {
		case p < 30:
			d.key(named[r.Intn(len(named))])
		case p < 55:
			d.key(letters[r.Intn(len(letters))])
		case p < 65:
			d.cmdline(cmds[r.Intn(len(cmds))])
		case p < 70:
			d.submit(cmds[r.Intn(len(cmds))])
		case p < 78:
			w, h := d.m.Width, d.m.Height
			if w <= 0 {
				w = 80
			}
			if h <= 0 {
				h = 24
			}
			d.click(r.Intn(w), r.Intn(h))
		case p < 82:
			x, y := pgSectorXY(engine.Coord{1 + r.Intn(8), 1 + r.Intn(8)})
			if d.m.ActiveModal == ModalGalacticChart {
				x, y = d.chartXY(engine.Coord{1 + r.Intn(8), 1 + r.Intn(8)})
			}
			if r.Intn(2) == 0 {
				d.click(x, y)
			} else {
				d.dblclick(x, y)
			}
		case p < 92:
			d.tick()
		case p < 95:
			s := sizes[r.Intn(len(sizes))]
			d.size(s[0], s[1])
		case p < 97:
			d.gameover(engine.GameOverReason(r.Intn(6)), float64(r.Intn(2000)))
		default:
			d.mouse(r.Intn(80), r.Intn(24), tea.MouseButton(r.Intn(4)), tea.MouseActionRelease)
		}
	}
}

// ---------------------------------------------------------------------------

func pgRunSession(t *testing.T, s pgSession, ascii bool, T time.Time, names map[string]bool, budget *int) string {
	tmp := t.TempDir()
	home := filepath.Join(tmp, "home")
	work := filepath.Join(tmp, "work")
	_ = os.MkdirAll(filepath.Join(home, ".config"), 0o755)
	_ = os.MkdirAll(work, 0o755)
	t.Setenv("HOME", home)
	t.Setenv("XDG_CONFIG_HOME", filepath.Join(home, ".config"))
	oldWd, _ := os.Getwd()
	if err := os.Chdir(work); err != nil {
		t.Fatal(err)
	}
	defer func() { _ = os.Chdir(oldWd) }()

	lipgloss.SetHasDarkBackground(true)
	if ascii {
		lipgloss.SetColorProfile(termenv.Ascii)
	} else {
		lipgloss.SetColorProfile(termenv.TrueColor)
	}
	theme.ResetDetectionCache()

	kv := pgParseSetup(s.setup)
	if n, ok := kv["saves"]; ok {
		for i := 1; i <= pgAtoi(n); i++ {
			g := engine.NewGame(int64(1000+i), engine.SkillGood, engine.LengthMedium)
			if err := engine.SaveGame(g, filepath.Join(work, fmt.Sprintf("MISSION%d.TRK", i))); err != nil {
				t.Fatal(err)
			}
		}
	}

	prof := "truecolor"
	if ascii {
		prof = "ascii"
	}
	var out bytes.Buffer
	fmt.Fprintf(&out, "session %s %s\nsetup %s\n", s.name, prof, s.setup)
	d := &pgDriver{
		m:      pgBuildModel(kv),
		out:    &out,
		step:   -1,
		ascii:  ascii,
		frames: s.frames && ascii,
		T:      T,
		work:   work,
		names:  names,
		budget: budget,
	}
	d.expand(d.m.Init())
	d.process()
	d.record("init")
	s.script(d)
	if d.failed != nil {
		t.Fatalf("session %s: %v", s.name, d.failed)
	}
	// Always store the final frame (unless it was just stored).
	if !(d.frames && strings.HasSuffix(out.String(), d.m.View()+"\n")) {
		d.finalFrame()
	}
	out.WriteString("end\n")
	return out.String()
}

func TestParityGen(t *testing.T) {
	if os.Getenv("SST_PARITYGEN") != "1" {
		t.Skip("set SST_PARITYGEN=1 to regenerate testdata/parity/tui")
	}
	time.Local = time.UTC
	t.Setenv("TZ", "UTC")
	t.Setenv("COLORFGBG", "15;0")
	t.Setenv("TERM_PROGRAM", "")
	_ = os.Unsetenv("TERM_PROGRAM")

	now := time.Now().UTC()
	T := time.Date(now.Year(), now.Month(), now.Day(), 12, 0, 0, 0, time.UTC)

	outDir, _ := filepath.Abs(filepath.Join("..", "..", "testdata", "parity", "tui"))
	if err := os.MkdirAll(outDir, 0o755); err != nil {
		t.Fatal(err)
	}

	names := map[string]bool{}
	budgets := map[string]int{"games": 1_100_000, "modals": 1_100_000, "scenarios": 600_000, "tour": 600_000, "mash": 100_000}
	files := pgSessions()
	order := []string{"games", "modals", "scenarios", "tour", "mash"}
	if only := os.Getenv("SST_PARITYONLY"); only != "" {
		order = strings.Split(only, ",")
	}
	totalSessions, totalSteps, totalFrames, totalBytes := 0, 0, 0, 0
	budget := 0
	for _, f := range order {
		budget = budgets[f]
		var buf bytes.Buffer
		fmt.Fprintf(&buf, "date %s\nclock_unix_nano %d\n", T.Format("2006-01-02"), T.UnixNano())
		for _, s := range files[f] {
			for _, ascii := range []bool{true, false} {
				// Each session gets an equal share of the file's frame budget.
				sb := budgets[f] / len(files[f])
				buf.WriteString(pgRunSession(t, s, ascii, T, names, &sb))
				budget -= budgets[f]/len(files[f]) - sb
				totalSessions++
			}
		}
		data := buf.Bytes()
		totalSteps += bytes.Count(data, []byte("\n= "))
		totalFrames += bytes.Count(data, []byte("\nframe "))
		totalBytes += len(data)
		if err := os.WriteFile(filepath.Join(outDir, f+".txt"), data, 0o644); err != nil {
			t.Fatal(err)
		}
	}
	var ns []string
	for n := range names {
		ns = append(ns, n)
	}
	sort.Strings(ns)
	fmt.Println("classified cmd funcs:")
	for _, n := range ns {
		fmt.Println("  " + n)
	}
	fmt.Printf("sessions=%d steps=%d frames=%d bytes=%d budget_left=%d\n", totalSessions, totalSteps, totalFrames, totalBytes, budget)
}
