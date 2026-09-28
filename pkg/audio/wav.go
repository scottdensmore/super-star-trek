package audio

import (
	"encoding/binary"
	"math"
	"sync"
)

const sampleRate = 8000

type wavCacheKey struct {
	id     SoundID
	volume int
}

var (
	wavCache   = make(map[wavCacheKey][]byte)
	rawCache   = make(map[SoundID][]int16)
	cacheMutex sync.RWMutex
)

// SynthesizeWav generates a standard 8kHz 16-bit mono RIFF/WAV byte buffer
// representing the specified procedural sound effect at full volume.
func SynthesizeWav(id SoundID) []byte {
	return SynthesizeWavWithVolume(id, 100)
}

// SynthesizeWavWithVolume generates a standard 8kHz 16-bit mono RIFF/WAV byte buffer
// representing the specified procedural sound effect scaled to the given volume (0-100).
func SynthesizeWavWithVolume(id SoundID, volume int) []byte {
	if volume < 0 {
		volume = 0
	}
	if volume > 100 {
		volume = 100
	}

	key := wavCacheKey{id: id, volume: volume}
	cacheMutex.RLock()
	if cached, ok := wavCache[key]; ok {
		cacheMutex.RUnlock()
		cp := make([]byte, len(cached))
		copy(cp, cached)
		return cp
	}
	cacheMutex.RUnlock()

	raw := rawSamples(id)
	scaled := make([]int16, len(raw))
	scale := float64(volume) / 100.0
	for i, s := range raw {
		v := float64(s) * scale
		if v > 32767.0 {
			v = 32767.0
		} else if v < -32768.0 {
			v = -32768.0
		}
		scaled[i] = int16(v)
	}

	wav := encodeWAV(scaled)

	cacheMutex.Lock()
	wavCache[key] = wav
	cacheMutex.Unlock()

	cp := make([]byte, len(wav))
	copy(cp, wav)
	return cp
}

func rawSamples(id SoundID) []int16 {
	cacheMutex.RLock()
	if cached, ok := rawCache[id]; ok {
		cacheMutex.RUnlock()
		return cached
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
	case SoundCloak:
		samples = synthesizeCloak()
	case SoundDecloak:
		samples = synthesizeDecloak()
	case SoundPlasmaLaunch:
		samples = synthesizePlasmaLaunch()
	case SoundPlasmaImpact:
		samples = synthesizePlasmaImpact()
	case SoundTholianWeb:
		samples = synthesizeTholianWeb()
	case SoundWebBreached:
		samples = synthesizeWebBreached()
	case SoundPointDefense:
		samples = synthesizePointDefense()
	case SoundCommChime:
		samples = synthesizeCommChime()
	case SoundComputerBeep:
		samples = synthesizeComputerBeep()
	default:
		samples = synthesizeBeep()
	}

	cacheMutex.Lock()
	rawCache[id] = samples
	cacheMutex.Unlock()

	return samples
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

// synthesizeCloak produces a resonant exponential downward sine sweep (1200 Hz down to 220 Hz over 450ms)
// with 12 Hz amplitude modulation simulating phased cloaking fields.
func synthesizeCloak() []int16 {
	duration := 0.45 // 450 ms
	numSamples := int(float64(sampleRate) * duration)
	samples := make([]int16, numSamples)

	f0 := 1200.0
	f1 := 220.0
	k := math.Log(f1/f0) / duration

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		phi := 2.0 * math.Pi * f0 * (math.Exp(k*t) - 1.0) / k
		am := 0.65 + 0.35*math.Sin(2.0*math.Pi*12.0*t)
		env := math.Pow(1.0-(t/duration), 0.8)
		if t < 0.02 {
			env *= (t / 0.02)
		}
		val := (0.75*math.Sin(phi) + 0.25*math.Sin(2.0*phi)) * am * env
		samples[i] = clampSample(val)
	}
	return samples
}

// synthesizeDecloak produces a rising pitch sweep (200 Hz up to 1400 Hz over 400ms)
// with a metallic square-wave harmonic burst upon phase lock.
func synthesizeDecloak() []int16 {
	duration := 0.40 // 400 ms
	numSamples := int(float64(sampleRate) * duration)
	samples := make([]int16, numSamples)

	f0 := 200.0
	f1 := 1400.0

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		phi := 2.0 * math.Pi * (f0*t + 0.5*(f1-f0)*t*t/duration)
		sq := 1.0
		if math.Sin(3.0*phi) < 0 {
			sq = -1.0
		}
		burstEnv := 0.0
		if t > 0.25 {
			burstEnv = math.Sin(math.Pi * (t - 0.25) / 0.15)
		}
		env := 1.0
		if t < 0.03 {
			env = t / 0.03
		} else if t > duration-0.04 {
			env = (duration - t) / 0.04
		}
		val := (0.65*math.Sin(phi) + 0.25*math.Sin(2.0*phi) + 0.35*sq*burstEnv) * env
		samples[i] = clampSample(val)
	}
	return samples
}

// synthesizePlasmaLaunch produces a low-frequency pulsing drone (85 Hz modulated at 12 Hz) with resonant hiss.
func synthesizePlasmaLaunch() []int16 {
	duration := 0.50 // 500 ms
	numSamples := int(float64(sampleRate) * duration)
	samples := make([]int16, numSamples)

	var rng uint32 = 0x543210
	var hissLP float64

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		drone := 0.75*math.Sin(2.0*math.Pi*85.0*t) + 0.25*math.Sin(2.0*math.Pi*170.0*t)
		droneMod := 0.6 + 0.4*math.Sin(2.0*math.Pi*12.0*t)

		rng = (rng*1103515245 + 12345) & 0x7fffffff
		noise := float64(rng%65536)/32768.0 - 1.0

		hissLP += 0.4 * (noise - hissLP)
		hiss := noise - hissLP

		env := math.Pow(1.0-(t/duration), 1.2)
		if t < 0.03 {
			env *= (t / 0.03)
		}
		val := (0.6*drone*droneMod + 0.4*hiss) * env
		samples[i] = clampSample(val)
	}
	return samples
}

// synthesizePlasmaImpact produces a dual-phase impact: 100ms high-energy electrical sizzle
// (white noise modulated by 400 Hz sine) followed by 500ms sub-bass exponential hull rumble (55 Hz).
func synthesizePlasmaImpact() []int16 {
	duration := 0.60 // 600 ms (100ms sizzle + 500ms rumble)
	numSamples := int(float64(sampleRate) * duration)
	samples := make([]int16, numSamples)

	var rng uint32 = 0x98765432
	var lpRumble float64

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		rng = (rng*1103515245 + 12345) & 0x7fffffff
		noise := float64(rng%65536)/32768.0 - 1.0

		var val float64
		if t < 0.10 {
			// Phase 1: High-energy electrical sizzle
			sizzleMod := 0.5 + 0.5*math.Sin(2.0*math.Pi*400.0*t)
			sizzleEnv := 1.0 - (t / 0.10)
			val = noise * sizzleMod * sizzleEnv * 1.2
		} else {
			// Phase 2: Sub-bass exponential hull rumble (55 Hz)
			t2 := t - 0.10
			rumbleEnv := math.Exp(-5.0 * t2)
			phiRumble := 2.0 * math.Pi * 55.0 * t2
			lpRumble += 0.1 * (noise - lpRumble)
			rumble := 0.75*math.Sin(phiRumble) + 0.25*math.Sin(2.0*phiRumble)
			val = (rumble*0.9 + lpRumble*0.3) * rumbleEnv
		}
		samples[i] = clampSample(val)
	}
	return samples
}

// synthesizeTholianWeb produces a crystalline arpeggio (harmonic chirps at 2400 Hz, 3200 Hz, 3600 Hz over 300ms).
func synthesizeTholianWeb() []int16 {
	duration := 0.30 // 300 ms
	numSamples := int(float64(sampleRate) * duration)
	samples := make([]int16, numSamples)

	notes := []float64{2400.0, 3200.0, 3600.0}
	noteDuration := duration / float64(len(notes))

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		noteIdx := int(t / noteDuration)
		if noteIdx >= len(notes) {
			noteIdx = len(notes) - 1
		}
		noteT := t - float64(noteIdx)*noteDuration
		freq := notes[noteIdx]

		phi := 2.0 * math.Pi * (freq*noteT + 200.0*noteT*noteT)
		env := math.Exp(-14.0 * noteT)
		val := (0.8*math.Sin(phi) + 0.2*math.Sin(2.0*phi)) * env
		samples[i] = clampSample(val)
	}
	return samples
}

// synthesizeWebBreached produces a crystalline fracture sound (dissonant cluster 3500 Hz + 3820 Hz decaying into white noise burst).
func synthesizeWebBreached() []int16 {
	duration := 0.35 // 350 ms
	numSamples := int(float64(sampleRate) * duration)
	samples := make([]int16, numSamples)

	var rng uint32 = 0xfe4321
	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		rng = (rng*1103515245 + 12345) & 0x7fffffff
		noise := float64(rng%65536)/32768.0 - 1.0

		cluster := 0.5*math.Sin(2.0*math.Pi*3500.0*t) + 0.5*math.Sin(2.0*math.Pi*3820.0*t)
		clusterEnv := math.Exp(-12.0 * t)
		noiseEnv := math.Exp(-6.0*t) * (1.0 - t/duration)

		val := cluster*clusterEnv*0.8 + noise*noiseEnv*0.7
		samples[i] = clampSample(val)
	}
	return samples
}

// synthesizePointDefense produces rapid 80ms pulsed high-frequency phaser beam bursts at 1800 Hz.
func synthesizePointDefense() []int16 {
	duration := 0.08 // 80 ms
	numSamples := int(float64(sampleRate) * duration)
	samples := make([]int16, numSamples)

	burstDuration := 0.02 // 4 bursts of 20ms each
	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		burstIdx := int(t / burstDuration)
		burstT := t - float64(burstIdx)*burstDuration

		var val float64
		if burstT < 0.015 {
			bEnv := 1.0 - (burstT / 0.015)
			phi := 2.0 * math.Pi * (1900.0*burstT - 0.5*200.0*burstT*burstT/0.015)
			val = (0.8*math.Sin(phi) + 0.2*math.Sin(2.0*phi)) * bEnv
		}
		samples[i] = clampSample(val)
	}
	return samples
}

// synthesizeCommChime produces the Starfleet bridge chime: 120ms at 987 Hz (B5) followed by 250ms at 1318 Hz (E6).
func synthesizeCommChime() []int16 {
	duration := 0.37 // 120ms + 250ms = 370ms
	numSamples := int(float64(sampleRate) * duration)
	samples := make([]int16, numSamples)

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		var freq, noteT, decayRate float64
		if t < 0.12 {
			freq = 987.0
			noteT = t
			decayRate = 7.0
		} else {
			freq = 1318.0
			noteT = t - 0.12
			decayRate = 5.0
		}
		decay := math.Exp(-decayRate * noteT)
		phi := 2.0 * math.Pi * freq * noteT
		val := (0.85*math.Sin(phi) + 0.15*math.Sin(2.0*phi)) * decay
		samples[i] = clampSample(val)
	}
	return samples
}

// synthesizeComputerBeep produces a short 60ms 1500 Hz acknowledgement beep.
func synthesizeComputerBeep() []int16 {
	duration := 0.06 // 60 ms
	numSamples := int(float64(sampleRate) * duration)
	samples := make([]int16, numSamples)

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		env := 1.0 - (t / duration)
		if t < 0.005 {
			env = (t / 0.005) * env
		}
		phi := 2.0 * math.Pi * 1500.0 * t
		val := math.Sin(phi) * env
		samples[i] = clampSample(val)
	}
	return samples
}

