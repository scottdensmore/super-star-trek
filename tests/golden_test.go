// Package tests provides integration and golden parity tests for Super Star Trek.
package tests

import (
	"bytes"
	"testing"

	"github.com/scottdensmore/super-star-trek/pkg/classic"
	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

func TestGoldenParityBasics(t *testing.T) {
	// Replay a deterministic game sequence
	game := engine.NewGame(12345, engine.SkillGood, engine.LengthMedium)
	var out bytes.Buffer
	sub := classic.NewTeletypeSubscriber(&out)

	events, err := game.Dispatch(engine.ActionShields{Amount: 500})
	if err != nil {
		t.Fatalf("action failed: %v", err)
	}

	for _, e := range events {
		sub.HandleEvent(e)
	}

	expected := "Energy: 4500  Shields: 500\n"
	if out.String() != expected {
		t.Fatalf("expected %q, got %q", expected, out.String())
	}
}
