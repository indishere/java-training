package main

import (
	"image/color"
	"math"

	"github.com/hajimehoshi/ebiten/v2"
)

const windowWidth = 400
const windowHeight = 300

// Game is the program Ebitengine runs.
type Game struct{}

// The Java program has no input or changing state, so there is nothing to update.
func (g *Game) Update() error {
	return nil
}

// Draw is the Go equivalent of Java's paint method.
func (g *Game) Draw(screen *ebiten.Image) {
	screen.Fill(color.White)

	centerX := windowWidth / 2
	centerY := windowHeight / 2
	faceSize := 120
	faceRadius := faceSize / 2

	drawFilledCircle(screen, centerX, centerY, faceRadius, color.RGBA{255, 255, 0, 255})
	drawCircleOutline(screen, centerX, centerY, faceRadius, color.Black)

	drawFilledCircle(screen, centerX-15, centerY-15, 5, color.Black)
	drawFilledCircle(screen, centerX+15, centerY-15, 5, color.Black)

	// Same position and arc as Java's drawArc(..., 210, 120).
	drawSmile(screen, centerX, centerY)
}

// Layout keeps the drawing area at 400 by 300 pixels.
func (g *Game) Layout(outsideWidth, outsideHeight int) (int, int) {
	return windowWidth, windowHeight
}

func drawFilledCircle(screen *ebiten.Image, centerX, centerY, radius int, fillColor color.Color) {
	for y := centerY - radius; y <= centerY+radius; y++ {
		for x := centerX - radius; x <= centerX+radius; x++ {
			distanceSquared := (x-centerX)*(x-centerX) + (y-centerY)*(y-centerY)
			if distanceSquared <= radius*radius {
				screen.Set(x, y, fillColor)
			}
		}
	}
}

func drawCircleOutline(screen *ebiten.Image, centerX, centerY, radius int, lineColor color.Color) {
	for y := centerY - radius; y <= centerY+radius; y++ {
		for x := centerX - radius; x <= centerX+radius; x++ {
			distanceSquared := (x-centerX)*(x-centerX) + (y-centerY)*(y-centerY)
			if math.Abs(float64(distanceSquared-radius*radius)) < float64(radius*2) {
				screen.Set(x, y, lineColor)
			}
		}
	}
}

func drawSmile(screen *ebiten.Image, centerX, centerY int) {
	for degrees := 210; degrees <= 330; degrees++ {
		angle := float64(degrees) * math.Pi / 180
		x := centerX + int(30*math.Cos(angle))
		y := centerY - 10 - int(20*math.Sin(angle))

		for offsetY := -1; offsetY <= 1; offsetY++ {
			for offsetX := -1; offsetX <= 1; offsetX++ {
				screen.Set(x+offsetX, y+offsetY, color.RGBA{255, 0, 0, 255})
			}
		}
	}
}

func main() {
	// These lines replace the Java JFrame setup.
	ebiten.SetWindowTitle("Frogger")
	ebiten.SetWindowSize(windowWidth, windowHeight)
	ebiten.SetWindowResizingMode(ebiten.WindowResizingModeEnabled)

	if err := ebiten.RunGame(&Game{}); err != nil {
		panic(err)
	}
}
