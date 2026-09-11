package classic

import (
	"bytes"
	"strings"
	"testing"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

func TestTeletypeEventFormatting(t *testing.T) {
	var buf bytes.Buffer
	sub := NewTeletypeSubscriber(&buf)

	sub.HandleEvent(engine.EventShieldTransfer{
		NewShields: 1500,
		NewEnergy:  3500,
	})

	output := buf.String()
	if !strings.Contains(output, "Shields: 1500") || !strings.Contains(output, "Energy: 3500") {
		t.Fatalf("unexpected subscriber format: %q", output)
	}
}

func TestTeletypeTorpedoEvents(t *testing.T) {
	t.Run("torpedo fired", func(t *testing.T) {
		var buf bytes.Buffer
		sub := NewTeletypeSubscriber(&buf)

		sub.HandleEvent(engine.EventTorpedoFired{
			Origin: engine.Coord{4, 1},
			Angle:  1.57,
		})

		expected := "Track: course 1.57\n"
		if buf.String() != expected {
			t.Fatalf("expected %q, got %q", expected, buf.String())
		}
	})

	t.Run("torpedo hit destroyed", func(t *testing.T) {
		var buf bytes.Buffer
		sub := NewTeletypeSubscriber(&buf)

		sub.HandleEvent(engine.EventTorpedoHit{
			Target:    engine.Coord{4, 7},
			Entity:    engine.EntityKlingon,
			Damage:    500,
			Destroyed: true,
		})

		expected := "*** Klingon destroyed ***\n"
		if buf.String() != expected {
			t.Fatalf("expected %q, got %q", expected, buf.String())
		}
	})

	t.Run("torpedo hit damaged", func(t *testing.T) {
		var buf bytes.Buffer
		sub := NewTeletypeSubscriber(&buf)

		sub.HandleEvent(engine.EventTorpedoHit{
			Target:    engine.Coord{4, 7},
			Entity:    engine.EntityKlingon,
			Damage:    250,
			Destroyed: false,
		})

		expected := "Hit: 250 units\n"
		if buf.String() != expected {
			t.Fatalf("expected %q, got %q", expected, buf.String())
		}
	})
}
