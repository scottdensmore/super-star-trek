package classic

import (
	"fmt"
	"io"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

// TeletypeSubscriber receives typed engine events and formats them as classic teletype output.
type TeletypeSubscriber struct {
	w io.Writer
}

// NewTeletypeSubscriber creates a new TeletypeSubscriber writing to w.
func NewTeletypeSubscriber(w io.Writer) *TeletypeSubscriber {
	return &TeletypeSubscriber{w: w}
}

// HandleEvent formats and prints an engine event to the configured writer.
func (s *TeletypeSubscriber) HandleEvent(e engine.Event) {
	switch evt := e.(type) {
	case engine.EventShieldTransfer:
		_, _ = fmt.Fprintf(s.w, "Energy: %.0f  Shields: %.0f\n", evt.NewEnergy, evt.NewShields)
	case engine.EventTorpedoFired:
		_, _ = fmt.Fprintf(s.w, "Track: course %.2f\n", evt.Angle)
	case engine.EventTorpedoHit:
		if evt.Destroyed {
			_, _ = fmt.Fprintf(s.w, "*** Klingon destroyed ***\n")
		} else {
			_, _ = fmt.Fprintf(s.w, "Hit: %.0f units\n", evt.Damage)
		}
	}
}
