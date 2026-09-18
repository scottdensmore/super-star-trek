package anim

import "github.com/scottdensmore/super-star-trek/pkg/engine"

// BresenhamLine returns a slice of grid coordinates from start to end inclusive
// using Bresenham's integer line algorithm.
func BresenhamLine(start, end engine.Coord) []engine.Coord {
	var points []engine.Coord
	x0, y0 := start.Col(), start.Row()
	x1, y1 := end.Col(), end.Row()

	dx := abs(x1 - x0)
	dy := -abs(y1 - y0)
	sx := 1
	if x0 > x1 {
		sx = -1
	}
	sy := 1
	if y0 > y1 {
		sy = -1
	}
	err := dx + dy

	for {
		points = append(points, engine.Coord{y0, x0})
		if x0 == x1 && y0 == y1 {
			break
		}
		e2 := 2 * err
		if e2 >= dy {
			err += dy
			x0 += sx
		}
		if e2 <= dx {
			err += dx
			y0 += sy
		}
	}
	return points
}

func abs(v int) int {
	if v < 0 {
		return -v
	}
	return v
}
