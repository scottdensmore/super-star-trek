package engine

// Dispatch executes the specified action against the game state, applying mutations
// and returning an event slice detailing all resulting occurrences.
func (g *GameState) Dispatch(action Action) ([]Event, error) {
	return action.Execute(g)
}
