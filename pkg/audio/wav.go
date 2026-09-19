package audio

import (
	"encoding/binary"
	"math"
	"sync"
)

const sampleRate = 8000

var (
	wavCache   = make(map[SoundID][]byte)
	cacheMutex sync.RWMutex
)

// SynthesizeWav generates a standard 8kHz 16-bit mono RIFF/WAV byte buffer
// representing the specified procedural sound effect.
func SynthesizeWav(id SoundID) []byte {
	cacheMutex.RLock()
	if cached, ok := wavCache[id]; ok {
		cacheMutex.RUnlock()
		cp := make([]byte, len(cached))
		copy(cp, cached)
		return cp
	}
	cacheMutex.RUnlock()

	var samples []int16
	switch id {
	case SoundPhaser:
		samples = synthesizePhaser()
	case SoundTorpedoLaunch:
		samples = synthesizeTorpedoLaunch()
	case SoundExplosion:
		samples = synthesizeExplosion()
	case SoundRedAlert:
		samples = synthesizeRedAlert()
	case SoundDock:
		samples = synthesizeDock()
	case SoundWarp:
		samples = synthesizeWarp()
	case SoundDamage:
		samples = synthesizeDamage()
	case SoundShields:
		samples = synthesizeShields()
	case SoundVictory:
		samples = synthesizeVictory()
	case SoundDefeat:
		samples = synthesizeDefeat()
	default:
		samples = synthesizeBeep()
	}

	wav := encodeWAV(samples)

	cacheMutex.Lock()
	wavCache[id] = wav
	cacheMutex.Unlock()

	cp := make([]byte, len(wav))
	copy(cp, wav)
	return cp
}

func encodeWAV(samples []int16) []byte {
	numSamples := len(samples)
	dataLen := numSamples * 2
	riffSize := 36 + dataLen
	buf := make([]byte, 44+dataLen)

	// RIFF header
	copy(buf[0:4], "RIFF")
	binary.LittleEndian.PutUint32(buf[4:8], uint32(riffSize))
	copy(buf[8:12], "WAVE")

	// fmt chunk
	copy(buf[12:16], "fmt ")
	binary.LittleEndian.PutUint32(buf[16:20], 16)      // subchunk1 size (16 for PCM)
	binary.LittleEndian.PutUint16(buf[20:22], 1)       // audio format (1 = PCM)
	binary.LittleEndian.PutUint16(buf[22:24], 1)       // num channels (1 = mono)
	binary.LittleEndian.PutUint32(buf[24:28], sampleRate)
	binary.LittleEndian.PutUint32(buf[28:32], sampleRate*2) // byte rate (sampleRate * channels * bytesPerSample)
	binary.LittleEndian.PutUint16(buf[32:34], 2)       // block align (channels * bytesPerSample)
	binary.LittleEndian.PutUint16(buf[34:36], 16)      // bits per sample

	// data chunk
	copy(buf[36:40], "data")
	binary.LittleEndian.PutUint32(buf[40:44], uint32(dataLen))

	for i, s := range samples {
		binary.LittleEndian.PutUint16(buf[44+i*2:46+i*2], uint16(s))
	}

	return buf
}

// clampSample limits a float64 audio sample to int16 range [-32767, 32767].
func clampSample(v float64) int16 {
	if v > 1.0 {
		v = 1.0
	} else if v < -1.0 {
		v = -1.0
	}
	return int16(v * 32760.0)
}

// synthesizePhaser produces a rapid downward frequency sweep (1200 Hz -> 200 Hz).
func synthesizePhaser() []int16 {
	duration := 0.25 // 250 ms
	numSamples := int(float64(sampleRate) * duration)
	samples := make([]int16, numSamples)

	f0 := 1200.0
	f1 := 200.0

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		// Linear frequency sweep: f(t) = f0 + (f1 - f0) * (t / D)
		// Phase: phi(t) = 2 * pi * (f0 * t + 0.5 * (f1 - f0) * t^2 / D)
		phi := 2.0 * math.Pi * (f0*t + 0.5*(f1-f0)*t*t/duration)
		env := 1.0 - (t / duration)
		val := (0.7*math.Sin(phi) + 0.3*math.Sin(2.0*phi)) * env
		samples[i] = clampSample(val)
	}
	return samples
}

// synthesizeTorpedoLaunch produces a rising whistling chirp (500 Hz -> 1200 Hz).
func synthesizeTorpedoLaunch() []int16 {
	duration := 0.20 // 200 ms
	numSamples := int(float64(sampleRate) * duration)
	samples := make([]int16, numSamples)

	f0 := 500.0
	f1 := 1200.0

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		phi := 2.0 * math.Pi * (f0*t + 0.5*(f1-f0)*t*t/duration)
		env := math.Pow(1.0-(t/duration), 1.2)
		val := (0.8*math.Sin(phi) + 0.2*math.Sin(3.0*phi)) * env
		samples[i] = clampSample(val)
	}
	return samples
}

// synthesizeExplosion produces a deterministic white noise burst with low-pass decay.
func synthesizeExplosion() []int16 {
	duration := 0.60 // 600 ms
	numSamples := int(float64(sampleRate) * duration)
	samples := make([]int16, numSamples)

	var rng uint32 = 0x12345678
	var lp float64

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		// Deterministic LCG
		rng = (rng*1103515245 + 12345) & 0x7fffffff
		rawNoise := float64(rng%65536)/32768.0 - 1.0

		// Low-pass filter with falling cutoff frequency
		progress := t / duration
		alpha := 0.35*(1.0-progress) + 0.03
		lp += alpha * (rawNoise - lp)

		env := math.Pow(1.0-progress, 2.0)
		val := lp * env * 1.5
		samples[i] = clampSample(val)
	}
	return samples
}

// synthesizeRedAlert produces a two-tone pulsing warble / siren.
func synthesizeRedAlert() []int16 {
	duration := 0.60 // 600 ms
	numSamples := int(float64(sampleRate) * duration)
	samples := make([]int16, numSamples)

	modFreq := 4.0 // 4 Hz modulation
	centerFreq := 700.0
	devFreq := 150.0

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		// Phase: phi(t) = 2*pi*(centerFreq * t - (devFreq / (2*pi*modFreq)) * cos(2*pi*modFreq * t))
		phi := 2.0*math.Pi*centerFreq*t - (devFreq/modFreq)*math.Cos(2.0*math.Pi*modFreq*t)
		val := 0.7*math.Sin(phi) + 0.3*math.Sin(3.0*phi)
		samples[i] = clampSample(val)
	}
	return samples
}

// synthesizeDock produces an ascending major triad chime (C5, E5, G5).
func synthesizeDock() []int16 {
	duration := 0.60 // 600 ms
	numSamples := int(float64(sampleRate) * duration)
	samples := make([]int16, numSamples)

	notes := []float64{523.25, 659.25, 783.99} // C5, E5, G5
	noteDuration := duration / 3.0

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		noteIdx := int(t / noteDuration)
		if noteIdx >= len(notes) {
			noteIdx = len(notes) - 1
		}
		noteT := t - float64(noteIdx)*noteDuration
		freq := notes[noteIdx]

		phi := 2.0 * math.Pi * freq * noteT
		env := math.Exp(-7.0 * noteT)
		val := (0.75*math.Sin(phi) + 0.25*math.Sin(2.0*phi)) * env
		samples[i] = clampSample(val)
	}
	return samples
}

// synthesizeWarp produces a low-frequency accelerating drone (55 Hz -> 165 Hz).
func synthesizeWarp() []int16 {
	duration := 0.70 // 700 ms
	numSamples := int(float64(sampleRate) * duration)
	samples := make([]int16, numSamples)

	f0 := 55.0
	f1 := 165.0

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		phi := 2.0 * math.Pi * (f0*t + 0.5*(f1-f0)*t*t/duration)
		env := 1.0
		if t < 0.05 {
			env = t / 0.05
		} else if t > duration-0.05 {
			env = (duration - t) / 0.05
		}
		val := (0.6*math.Sin(phi) + 0.3*math.Sin(2.0*phi) + 0.1*math.Sin(3.0*phi)) * env
		samples[i] = clampSample(val)
	}
	return samples
}

// synthesizeDamage produces a harsh crunchy impact noise with low-frequency square wave.
func synthesizeDamage() []int16 {
	duration := 0.35 // 350 ms
	numSamples := int(float64(sampleRate) * duration)
	samples := make([]int16, numSamples)

	var rng uint32 = 0xabcdef01
	carrierFreq := 110.0

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		rng = (rng*1103515245 + 12345) & 0x7fffffff
		noise := float64(rng%65536)/32768.0 - 1.0

		square := 1.0
		if math.Sin(2.0*math.Pi*carrierFreq*t) < 0 {
			square = -1.0
		}

		env := math.Pow(1.0-(t/duration), 2.5)
		val := (0.6*noise + 0.4*square) * env
		samples[i] = clampSample(val)
	}
	return samples
}

// synthesizeShields produces a resonant flare shimmer with frequency modulation.
func synthesizeShields() []int16 {
	duration := 0.40 // 400 ms
	numSamples := int(float64(sampleRate) * duration)
	samples := make([]int16, numSamples)

	carrier := 440.0
	modRate := 25.0

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		phi := 2.0*math.Pi*carrier*t + 2.5*math.Sin(2.0*math.Pi*modRate*t)
		env := math.Exp(-5.0 * t)
		if t < 0.04 {
			env = (t / 0.04) * env
		}
		val := math.Sin(phi) * env
		samples[i] = clampSample(val)
	}
	return samples
}

// synthesizeVictory produces an ascending 4-note fanfare (C5, E5, G5, C6).
func synthesizeVictory() []int16 {
	notes := []float64{523.25, 659.25, 783.99, 1046.50} // C5, E5, G5, C6
	noteDuration := 0.15                                // 150 ms per note
	totalDuration := noteDuration * float64(len(notes))
	numSamples := int(float64(sampleRate) * totalDuration)
	samples := make([]int16, numSamples)

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		noteIdx := int(t / noteDuration)
		if noteIdx >= len(notes) {
			noteIdx = len(notes) - 1
		}
		noteT := t - float64(noteIdx)*noteDuration
		freq := notes[noteIdx]

		phi := 2.0 * math.Pi * freq * noteT
		env := math.Exp(-6.0 * noteT)
		val := (0.8*math.Sin(phi) + 0.2*math.Sin(2.0*phi)) * env
		samples[i] = clampSample(val)
	}
	return samples
}

// synthesizeDefeat produces a descending mournful slide (420 Hz -> 110 Hz).
func synthesizeDefeat() []int16 {
	duration := 0.70 // 700 ms
	numSamples := int(float64(sampleRate) * duration)
	samples := make([]int16, numSamples)

	f0 := 420.0
	f1 := 110.0

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		phi := 2.0 * math.Pi * (f0*t + 0.5*(f1-f0)*t*t/duration)
		env := math.Pow(1.0-(t/duration), 1.5)
		val := math.Sin(phi) * env
		samples[i] = clampSample(val)
	}
	return samples
}

// synthesizeBeep produces a short 440 Hz beep fallback.
func synthesizeBeep() []int16 {
	duration := 0.10 // 100 ms
	numSamples := int(float64(sampleRate) * duration)
	samples := make([]int16, numSamples)

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		phi := 2.0 * math.Pi * 440.0 * t
		env := 1.0 - (t / duration)
		val := math.Sin(phi) * env
		samples[i] = clampSample(val)
	}
	return samples
}
