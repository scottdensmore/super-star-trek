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

// normalizeSamples scales a float64 waveform buffer so its peak amplitude equals
// targetPeak (in [0, 1], where 0.285 corresponds to ~9338 int16 peak), ensuring
// every sound is clearly audible on laptop speakers while remaining warm and subtle.
func normalizeSamples(buf []float64, targetPeak float64) []int16 {
	var maxAbs float64
	for _, v := range buf {
		a := math.Abs(v)
		if a > maxAbs {
			maxAbs = a
		}
	}
	out := make([]int16, len(buf))
	if maxAbs < 1e-9 {
		return out
	}
	scale := targetPeak / maxAbs
	for i, v := range buf {
		out[i] = clampSample(v * scale)
	}
	out[0] = 0
	if len(out) > 1 {
		out[len(out)-1] = 0
	}
	return out
}

// smoothEnvelope returns a raised-cosine attack and release envelope multiplier in [0, 1].
func smoothEnvelope(t, duration, attack, release float64) float64 {
	if t <= 0 || t >= duration {
		return 0
	}
	env := 1.0
	if attack > 0 && t < attack {
		env = 0.5 * (1.0 - math.Cos(math.Pi*(t/attack)))
	} else if release > 0 && t > duration-release {
		rem := (duration - t) / release
		env = 0.5 * (1.0 - math.Cos(math.Pi*rem))
	}
	return env
}

// synthesizePhaser produces the authentic Star Trek TOS Enterprise Ship Phaser beam
// (modeled directly on the spectral profile of the TOS "Enterprise Phasers Firing" sound effect):
//   - Twin oscillating tape-feedback carriers centered at ~1360 Hz (1280-1440 Hz) and
//     ~2550 Hz (2420-2680 Hz) sweeping at 8 Hz with 26 Hz flutter and pink-noise FM jitter
//   - Heavy band-filtered plasma/ion beam hiss (1200-2800 Hz) giving the characteristic
//     searing starship phaser roar/screech (ZCR ~2150 Hz, tonality ~0.22)
//   - Warm 560 Hz / 940 Hz emitter body underneath so it sounds full on laptop speakers.
func synthesizePhaser() []int16 {
	duration := 0.65 // 650 ms (matches the exact 650ms burst in tos_ship_phaser_1)
	numSamples := int(float64(sampleRate) * duration)
	buf := make([]float64, numSamples)

	var rng uint32 = 0x41a7f3d1
	var lpFast, lpSlow, lpPink float64
	var phase1, phase2, phaseBody float64
	dt := 1.0 / float64(sampleRate)

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)

		// Generate deterministic white noise and filter into pink/mid-band noise
		rng = (rng*1103515245 + 12345) & 0x7fffffff
		white := float64(rng%65536)/32768.0 - 1.0

		// Smooth pink-like noise for analog tape/Moog FM jitter (Ben Burtt technique)
		lpPink += 0.18 * (white - lpPink)

		// High-energy bandpass noise (1200 Hz - 2800 Hz) for the searing phaser beam hiss/roar
		lpFast += 0.68 * (white - lpFast)
		lpSlow += 0.26 * (white - lpSlow)
		beamHiss := lpFast - lpSlow

		// Carrier 1: ~1360 Hz sweeping between 1280 Hz and 1440 Hz at 8 Hz + 26 Hz ripple + noise FM
		f1 := 1360.0 + 80.0*math.Sin(2.0*math.Pi*8.0*t) + 28.0*math.Sin(2.0*math.Pi*26.0*t) + 95.0*lpPink
		phase1 += 2.0 * math.Pi * f1 * dt

		// Carrier 2: ~2550 Hz sweeping between 2420 Hz and 2680 Hz (the signature TOS ship phaser upper screech)
		f2 := 2550.0 + 130.0*math.Sin(2.0*math.Pi*8.0*t-0.85) + 42.0*math.Sin(2.0*math.Pi*26.0*t+0.5) + 140.0*lpPink
		phase2 += 2.0 * math.Pi * f2 * dt

		// Emitter body hum (~560 Hz + 940 Hz from tos_ship_phaser_3) for starship weapon weight
		fBody := 560.0 + 30.0*math.Sin(2.0*math.Pi*8.0*t)
		phaseBody += 2.0 * math.Pi * fBody * dt

		// Pulsing AM shimmer across the beam
		am := 0.86 + 0.14*math.Sin(2.0*math.Pi*26.0*t)
		tones := (0.44*math.Sin(phase1) + 0.44*math.Sin(phase2) + 0.12*math.Sin(phaseBody)) * am

		// Flat, high-energy sustained beam plateau with crisp 18ms attack and 55ms release
		env := smoothEnvelope(t, duration, 0.018, 0.055)
		buf[i] = (0.52*tones + 0.48*beamHiss*(0.85+0.15*math.Sin(2.0*math.Pi*16.0*t))) * env
	}
	return normalizeSamples(buf, 0.285)
}

// synthesizeTorpedoLaunch produces the classic 1966 TOS / retro dispersive spring-wire
// photon torpedo sound ("ptew-ew-ew-ew"): 5 overlapping downward pitch-chirp
// reflections (modeling a struck high-tension guy wire with tape slapback echo)
// plus a warm pneumatic launch tube punch over 720ms.
func synthesizeTorpedoLaunch() []int16 {
	duration := 0.72 // 720 ms so all dispersive spring echoes ring out clearly
	numSamples := int(float64(sampleRate) * duration)
	buf := make([]float64, numSamples)

	// 5 dispersive spring-reverb chirp taps (delay, weight, startFreq, endFreq, tapDur)
	tapDelays := [5]float64{0.000, 0.105, 0.215, 0.330, 0.450}
	tapWeights := [5]float64{1.00, 0.82, 0.64, 0.48, 0.34}
	tapStartFreqs := [5]float64{1020.0, 900.0, 790.0, 700.0, 620.0}
	tapEndFreqs := [5]float64{150.0, 142.0, 135.0, 128.0, 120.0}
	tapDurs := [5]float64{0.19, 0.20, 0.21, 0.22, 0.24}

	var rng uint32 = 0x7b3c9e15
	var lp1, lp2 float64

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)

		// 1. Overlapping dispersive guy-wire / spring chirp reflections ("ptew-ew-ew-ew")
		var springSum float64
		for tap := 0; tap < 5; tap++ {
			dt := t - tapDelays[tap]
			tDur := tapDurs[tap]
			if dt >= 0 && dt < tDur {
				f0 := tapStartFreqs[tap]
				f1 := tapEndFreqs[tap]
				k := math.Log(f1/f0) / tDur
				phi := 2.0 * math.Pi * f0 * (math.Exp(k*dt) - 1.0) / k

				// Rich retro metallic spring harmonic series (1x, 2x, 3x)
				wave := 0.52*math.Sin(phi) + 0.32*math.Sin(2.0*phi) + 0.16*math.Sin(3.0*phi)
				tapEnv := smoothEnvelope(dt, tDur, 0.006, 0.055) * math.Exp(-1.8*(dt/tDur))
				springSum += wave * tapEnv * tapWeights[tap]
			}
		}

		// 2. Warm pneumatic tube launch punch at t=0..140ms (260 Hz -> 75 Hz with upper harmonics)
		var punch float64
		if t < 0.14 {
			phiPunch := 2.0 * math.Pi * (260.0*t - 660.0*t*t)
			pEnv := smoothEnvelope(t, 0.14, 0.004, 0.040) * math.Exp(-8.0*t)
			punch = (0.55*math.Sin(phiPunch) + 0.30*math.Sin(2.0*phiPunch) + 0.15*math.Sin(3.0*phiPunch)) * pEnv
		}

		// 3. Subtle low-passed pneumatic air burst
		rng = (rng*1103515245 + 12345) & 0x7fffffff
		noise := float64(rng%65536)/32768.0 - 1.0
		lp1 += 0.28 * (noise - lp1)
		lp2 += 0.28 * (lp1 - lp2)
		pneumatic := lp2 * math.Exp(-9.0*t)

		env := smoothEnvelope(t, duration, 0.006, 0.080)
		buf[i] = (0.76*springSum + 0.18*punch + 0.06*pneumatic) * env
	}
	return normalizeSamples(buf, 0.290)
}

// synthesizeExplosion produces a warm TOS deep-space detonation: a low-mid
// resonant hull boom (150 Hz -> 45 Hz with 2nd & 3rd harmonics) blended with warm low-pass rumble.
func synthesizeExplosion() []int16 {
	duration := 0.62 // 620 ms
	numSamples := int(float64(sampleRate) * duration)
	buf := make([]float64, numSamples)

	var rng uint32 = 0x12345678
	var lp1, lp2 float64

	f0 := 150.0
	f1 := 45.0

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		progress := t / duration

		// Deterministic LCG with two-pole warm low-pass filter
		rng = (rng*1103515245 + 12345) & 0x7fffffff
		rawNoise := float64(rng%65536)/32768.0 - 1.0

		alpha := 0.26*(1.0-progress) + 0.04
		lp1 += alpha * (rawNoise - lp1)
		lp2 += alpha * (lp1 - lp2)

		// Resonant detonation boom with audible upper harmonics (150-450 Hz)
		phiBoom := 2.0 * math.Pi * (f0*t + 0.5*(f1-f0)*t*t/duration)
		boom := 0.52*math.Sin(phiBoom) + 0.32*math.Sin(2.0*phiBoom) + 0.16*math.Sin(3.0*phiBoom)

		env := smoothEnvelope(t, duration, 0.008, 0.100) * math.Exp(-1.8*progress)
		buf[i] = (0.58*boom + 0.42*lp2*1.6) * env
	}
	return normalizeSamples(buf, 0.290)
}

// synthesizeRedAlert produces the Star Trek TOS "Condition Red" klaxon ("Reed Alert"):
// a warm upward-curving electronic reed sweep (440 Hz -> 860 Hz) over 620ms.
func synthesizeRedAlert() []int16 {
	duration := 0.62 // 620 ms
	numSamples := int(float64(sampleRate) * duration)
	buf := make([]float64, numSamples)

	var phase float64
	dt := 1.0 / float64(sampleRate)

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)

		// Characteristic TOS curved upward sweep over first 460ms, then gentle release
		sweepProgress := t / 0.46
		if sweepProgress > 1.0 {
			sweepProgress = 1.0
		}
		freq := 440.0 + 420.0*math.Pow(sweepProgress, 0.75)
		// Subtle 18 Hz electronic reed flutter
		freq += 8.0 * math.Sin(2.0*math.Pi*18.0*t)

		phase += 2.0 * math.Pi * freq * dt

		// Warm reed harmonic structure (fundamental + soft 2nd & 3rd harmonics)
		reed := 0.65*math.Sin(phase) + 0.24*math.Sin(2.0*phase) + 0.11*math.Sin(3.0*phase)

		env := smoothEnvelope(t, duration, 0.012, 0.095)
		if t > 0.46 {
			env *= math.Exp(-3.5 * (t - 0.46))
		}
		buf[i] = reed * env
	}
	return normalizeSamples(buf, 0.275)
}

// synthesizeDock produces a warm, low-pitched Starbase docking sequence over 750ms:
// a mechanical docking clamp latch and soft pneumatic airlock seal followed by a rich,
// low-register two-chord confirmation hum (A3 220 Hz -> E4 329.63 Hz with warm overtones
// so it is clearly audible on laptop speakers without being high-pitched).
func synthesizeDock() []int16 {
	duration := 0.75 // 750 ms
	numSamples := int(float64(sampleRate) * duration)
	buf := make([]float64, numSamples)

	var rng uint32 = 0x31415926
	var lp1, lp2 float64
	split := 0.30 // 300ms first docking latch + A3 hum, then 450ms E4/A3 fifth lock

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)

		// Soft two-pole low-passed pneumatic airlock pressure equalization hiss
		rng = (rng*1103515245 + 12345) & 0x7fffffff
		noise := float64(rng%65536)/32768.0 - 1.0
		lp1 += 0.22 * (noise - lp1)
		lp2 += 0.22 * (lp1 - lp2)
		pneumatic := lp2 * math.Exp(-4.0*t)

		var freq, noteT, noteDur, clampThud float64
		if t < split {
			freq = 220.00 // A3 (warm low register)
			noteT = t
			noteDur = split
			// First mechanical docking clamp latch (240 Hz -> 90 Hz with 2x harmonic for speaker clarity)
			phiClamp := 2.0 * math.Pi * (240.0*noteT - 280.0*noteT*noteT)
			clampThud = (0.60*math.Sin(phiClamp) + 0.40*math.Sin(2.0*phiClamp)) * math.Exp(-14.0*noteT)
		} else {
			freq = 329.63 // E4 (warm mellow fifth above A3)
			noteT = t - split
			noteDur = duration - split
			// Second docking ring seal lock (190 Hz -> 80 Hz)
			phiClamp := 2.0 * math.Pi * (190.0*noteT - 200.0*noteT*noteT)
			clampThud = (0.60*math.Sin(phiClamp) + 0.40*math.Sin(2.0*phiClamp)) * math.Exp(-16.0*noteT)
		}

		// Warm low-register confirmation tone + rich 1.5x/2x harmonics (220-660 Hz range)
		phi := 2.0 * math.Pi * freq * noteT
		tone := 0.50*math.Sin(phi) + 0.30*math.Sin(2.0*phi) + 0.20*math.Sin(1.5*phi)
		toneEnv := smoothEnvelope(noteT, noteDur, 0.015, 0.070) * math.Exp(-1.1*noteT)

		env := smoothEnvelope(t, duration, 0.008, 0.080)
		buf[i] = (0.72*tone*toneEnv + 0.20*clampThud + 0.08*pneumatic) * env
	}
	return normalizeSamples(buf, 0.280)
}

// synthesizeWarp produces an immediate, crisp Star Trek TOS impulse/warp drive engage pulse:
// a front-loaded harmonic turbine thrum (140 Hz -> 260 Hz body + 280 Hz -> 520 Hz TOS bridge
// turbine shimmer) with a 4ms attack over 420ms so it is heard the instant the ship moves.
func synthesizeWarp() []int16 {
	duration := 0.42 // 420 ms engage pulse
	numSamples := int(float64(sampleRate) * duration)
	buf := make([]float64, numSamples)

	f0 := 140.0
	f1 := 260.0
	k := math.Log(f1/f0) / duration

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		progress := t / duration

		phiBase := 2.0 * math.Pi * f0 * (math.Exp(k*t) - 1.0) / k
		phiShim := 2.0*phiBase + 1.4*math.Sin(2.0*math.Pi*18.0*t) // 280 -> 520 Hz TOS turbine shimmer

		// Immediate audible mid-range presence at t=0 with warm harmonic body
		hum := 0.42*math.Sin(phiBase) + 0.40*math.Sin(phiShim) + 0.18*math.Sin(3.0*phiBase)

		// Fast 4ms attack, sustained body with smooth release
		env := smoothEnvelope(t, duration, 0.004, 0.080) * math.Exp(-1.1*progress)
		buf[i] = hum * env
	}
	return normalizeSamples(buf, 0.275)
}

// synthesizeDamage produces the Star Trek TOS Klingon return-fire sequence over 580ms:
// Phase 1 (0..220ms): Incoming Klingon disruptor bolt (920 Hz -> 290 Hz warbled pulse)
// Phase 2 (190..580ms): Enterprise shield/hull impact (320 Hz -> 95 Hz resonant thud) + console spark crackle.
func synthesizeDamage() []int16 {
	duration := 0.58 // 580 ms
	numSamples := int(float64(sampleRate) * duration)
	buf := make([]float64, numSamples)

	var rng uint32 = 0xabcdef01
	var lpFast, lpSlow float64

	disruptorDur := 0.22
	df0 := 920.0
	df1 := 290.0
	dk := math.Log(df1/df0) / disruptorDur

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)

		var val float64

		// Phase 1: Incoming Klingon disruptor shot (0..220ms)
		if t < disruptorDur {
			dPhi := 2.0*math.Pi*df0*(math.Exp(dk*t)-1.0)/dk + 1.8*math.Sin(2.0*math.Pi*32.0*t)
			dAM := 0.74 + 0.26*math.Sin(2.0*math.Pi*32.0*t)
			disruptor := (0.55*math.Sin(dPhi) + 0.30*math.Sin(2.0*dPhi) + 0.15*math.Sin(3.0*dPhi)) * dAM
			dEnv := smoothEnvelope(t, disruptorDur, 0.006, 0.035)
			val += disruptor * dEnv * 0.95
		}

		// Phase 2: Enterprise shield/hull hit & bridge console sparks (190..580ms)
		if t >= 0.190 {
			it := t - 0.190
			iDur := duration - 0.190

			// Audible mid-bass shield/hull impact (320 Hz -> 95 Hz with 2nd & 3rd harmonics)
			phiHit := 2.0 * math.Pi * (320.0*it - 0.5*225.0*it*it/iDur)
			thud := (0.52*math.Sin(phiHit) + 0.32*math.Sin(2.0*phiHit) + 0.16*math.Sin(3.0*phiHit)) * math.Exp(-4.5*it)

			// Crisp band-filtered bridge console spark crackle
			rng = (rng*1103515245 + 12345) & 0x7fffffff
			noise := float64(rng%65536)/32768.0 - 1.0
			lpFast += 0.48 * (noise - lpFast)
			lpSlow += 0.14 * (noise - lpSlow)
			spark := (lpFast - lpSlow) * (0.75 + 0.25*math.Sin(2.0*math.Pi*26.0*it)) * math.Exp(-4.0*it)

			iEnv := smoothEnvelope(it, iDur, 0.006, 0.065)
			val += (0.65*thud + 0.45*spark) * iEnv
		}

		buf[i] = val
	}
	return normalizeSamples(buf, 0.290)
}

// synthesizeShields produces a warm Star Trek TOS deflector screen harmonic hum/swell
// (220 Hz + 330 Hz + 660 Hz with gentle 14 Hz phase shimmer) over 450ms.
func synthesizeShields() []int16 {
	duration := 0.45 // 450 ms
	numSamples := int(float64(sampleRate) * duration)
	buf := make([]float64, numSamples)

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		mod := 1.2 * math.Sin(2.0*math.Pi*14.0*t)
		phi1 := 2.0*math.Pi*220.0*t + 0.5*mod
		phi2 := 2.0*math.Pi*330.0*t + mod
		phi3 := 2.0*math.Pi*660.0*t + 1.5*mod

		am := 0.84 + 0.16*math.Sin(2.0*math.Pi*14.0*t)
		chord := 0.50*math.Sin(phi1) + 0.34*math.Sin(phi2) + 0.16*math.Sin(phi3)

		env := smoothEnvelope(t, duration, 0.012, 0.080) * math.Exp(-1.6*(t/duration))
		buf[i] = chord * am * env
	}
	return normalizeSamples(buf, 0.270)
}

// synthesizeVictory produces a warm, understated TOS Starfleet fanfare (A4, C#5, E5, A5).
func synthesizeVictory() []int16 {
	notes := []float64{440.00, 554.37, 659.25, 880.00} // A4, C#5, E5, A5
	noteDuration := 0.18                               // 180 ms per note
	totalDuration := noteDuration * float64(len(notes))
	numSamples := int(float64(sampleRate) * totalDuration)
	buf := make([]float64, numSamples)

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		noteIdx := int(t / noteDuration)
		if noteIdx >= len(notes) {
			noteIdx = len(notes) - 1
		}
		noteT := t - float64(noteIdx)*noteDuration
		freq := notes[noteIdx]

		phi := 2.0*math.Pi*freq*noteT + 0.4*math.Sin(2.0*math.Pi*5.5*noteT)
		env := smoothEnvelope(noteT, noteDuration, 0.012, 0.035) * math.Exp(-2.2*noteT)
		buf[i] = (0.75*math.Sin(phi) + 0.25*math.Sin(2.0*phi)) * env
	}
	return normalizeSamples(buf, 0.275)
}

// synthesizeDefeat produces a somber, subtle TOS low-register descending cadence (E4 -> C4 -> A3).
func synthesizeDefeat() []int16 {
	notes := []float64{329.63, 261.63, 220.00} // E4, C4, A3
	noteDuration := 0.25                       // 250 ms per note
	totalDuration := noteDuration * float64(len(notes))
	numSamples := int(float64(sampleRate) * totalDuration)
	buf := make([]float64, numSamples)

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		noteIdx := int(t / noteDuration)
		if noteIdx >= len(notes) {
			noteIdx = len(notes) - 1
		}
		noteT := t - float64(noteIdx)*noteDuration
		freq := notes[noteIdx]

		phi := 2.0 * math.Pi * freq * noteT
		env := smoothEnvelope(noteT, noteDuration, 0.018, 0.050) * math.Exp(-1.8*noteT)
		buf[i] = (0.70*math.Sin(phi) + 0.30*math.Sin(2.0*phi)) * env
	}
	return normalizeSamples(buf, 0.275)
}

// synthesizeBeep produces a soft, subtle 880 Hz fallback tone.
func synthesizeBeep() []int16 {
	duration := 0.16 // 160 ms
	numSamples := int(float64(sampleRate) * duration)
	buf := make([]float64, numSamples)

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		phi := 2.0 * math.Pi * 880.0 * t
		env := smoothEnvelope(t, duration, 0.010, 0.035)
		buf[i] = math.Sin(phi) * env
	}
	return normalizeSamples(buf, 0.250)
}

// synthesizeCloak produces the TOS ("Balance of Terror") Romulan cloaking veil:
// a smooth, ethereal detuned exponential downward sweep (960 Hz down to 190 Hz)
// with 11 Hz phase shimmer.
func synthesizeCloak() []int16 {
	duration := 0.55 // 550 ms
	numSamples := int(float64(sampleRate) * duration)
	buf := make([]float64, numSamples)

	f0 := 960.0
	f1 := 190.0
	k := math.Log(f1/f0) / duration

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		phi1 := 2.0 * math.Pi * f0 * (math.Exp(k*t) - 1.0) / k
		phi2 := 1.008 * phi1 // Subtle detuning for hollow phase-shift texture
		am := 0.75 + 0.25*math.Sin(2.0*math.Pi*11.0*t)

		env := smoothEnvelope(t, duration, 0.030, 0.090) * math.Pow(1.0-(t/duration), 0.45)
		buf[i] = (0.50*math.Sin(phi1) + 0.35*math.Sin(phi2) + 0.15*math.Sin(2.0*phi1)) * am * env
	}
	return normalizeSamples(buf, 0.275)
}

// synthesizeDecloak produces the reverse TOS Romulan uncloaking phase-in sweep
// (190 Hz up to 960 Hz) with a smooth resonant harmonic lock.
func synthesizeDecloak() []int16 {
	duration := 0.52 // 520 ms
	numSamples := int(float64(sampleRate) * duration)
	buf := make([]float64, numSamples)

	f0 := 190.0
	f1 := 960.0
	k := math.Log(f1/f0) / duration

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		phi1 := 2.0 * math.Pi * f0 * (math.Exp(k*t) - 1.0) / k
		phi2 := 1.008 * phi1
		am := 0.75 + 0.25*math.Sin(2.0*math.Pi*11.0*t)

		env := smoothEnvelope(t, duration, 0.035, 0.080)
		buf[i] = (0.50*math.Sin(phi1) + 0.35*math.Sin(phi2) + 0.15*math.Sin(2.0*phi1)) * am * env
	}
	return normalizeSamples(buf, 0.275)
}

// synthesizePlasmaLaunch produces an eerie, pulsing TOS Romulan plasma torpedo hum
// (110 Hz -> 165 Hz core + 440 Hz -> 660 Hz overtone modulated at 14 Hz) with soft hiss.
func synthesizePlasmaLaunch() []int16 {
	duration := 0.58 // 580 ms
	numSamples := int(float64(sampleRate) * duration)
	buf := make([]float64, numSamples)

	var rng uint32 = 0x543210
	var lpHiss float64

	f0 := 110.0
	f1 := 165.0

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		phi := 2.0 * math.Pi * (f0*t + 0.5*(f1-f0)*t*t/duration)
		drone := 0.50*math.Sin(phi) + 0.32*math.Sin(2.0*phi) + 0.18*math.Sin(4.0*phi)
		droneMod := 0.72 + 0.28*math.Sin(2.0*math.Pi*14.0*t)

		rng = (rng*1103515245 + 12345) & 0x7fffffff
		noise := float64(rng%65536)/32768.0 - 1.0
		lpHiss += 0.25 * (noise - lpHiss)

		env := smoothEnvelope(t, duration, 0.025, 0.085) * math.Exp(-1.2*(t/duration))
		buf[i] = (0.82*drone*droneMod + 0.18*lpHiss) * env
	}
	return normalizeSamples(buf, 0.280)
}

// synthesizePlasmaImpact produces a warm, subtle plasma energy wash followed by a deep
// 80 Hz -> 40 Hz hull resonance.
func synthesizePlasmaImpact() []int16 {
	duration := 0.62 // 620 ms
	numSamples := int(float64(sampleRate) * duration)
	buf := make([]float64, numSamples)

	var rng uint32 = 0x98765432
	var lpWash, lpRumble float64

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		rng = (rng*1103515245 + 12345) & 0x7fffffff
		noise := float64(rng%65536)/32768.0 - 1.0

		// Soft plasma wash at impact onset
		lpWash += 0.28 * (noise - lpWash)
		washMod := 0.70 + 0.30*math.Sin(2.0*math.Pi*22.0*t)
		wash := lpWash * washMod * math.Exp(-5.5*t)

		// Deep resonant hull rumble (90 Hz -> 42 Hz with 2x and 3x harmonics)
		phiRumble := 2.0 * math.Pi * (90.0*t - 48.0*t*t/duration)
		lpRumble += 0.10 * (noise - lpRumble)
		rumble := (0.58*math.Sin(phiRumble) + 0.28*math.Sin(2.0*phiRumble) + 0.14*math.Sin(3.0*phiRumble) + 0.20*lpRumble) * math.Exp(-2.8*t)

		env := smoothEnvelope(t, duration, 0.012, 0.085)
		buf[i] = (0.38*wash + 0.62*rumble) * env
	}
	return normalizeSamples(buf, 0.290)
}

// synthesizeTholianWeb produces a delicate, shimmering crystalline triad
// (D6 1174.66 Hz, F#6 1479.98 Hz, A6 1760.00 Hz — cleanly band-limited below Nyquist).
func synthesizeTholianWeb() []int16 {
	duration := 0.45 // 450 ms
	numSamples := int(float64(sampleRate) * duration)
	buf := make([]float64, numSamples)

	notes := []float64{1174.66, 1479.98, 1760.00} // D6, F#6, A6
	noteDuration := duration / float64(len(notes))

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		noteIdx := int(t / noteDuration)
		if noteIdx >= len(notes) {
			noteIdx = len(notes) - 1
		}
		noteT := t - float64(noteIdx)*noteDuration
		freq := notes[noteIdx]

		phi := 2.0*math.Pi*freq*noteT + 0.8*math.Sin(2.0*math.Pi*24.0*noteT)
		env := smoothEnvelope(noteT, noteDuration, 0.008, 0.030) * math.Exp(-4.0*noteT)
		buf[i] = (0.82*math.Sin(phi) + 0.18*math.Sin(2.0*phi)) * env
	}
	return normalizeSamples(buf, 0.265)
}

// synthesizeWebBreached produces a subtle crystalline harmonic release (1760 Hz + 1320 Hz
// sweeping down to 550 Hz) with a soft filtered dissipation sigh.
func synthesizeWebBreached() []int16 {
	duration := 0.46 // 460 ms
	numSamples := int(float64(sampleRate) * duration)
	buf := make([]float64, numSamples)

	var rng uint32 = 0xfe4321
	var lpNoise float64

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		rng = (rng*1103515245 + 12345) & 0x7fffffff
		noise := float64(rng%65536)/32768.0 - 1.0
		lpNoise += 0.25 * (noise - lpNoise)

		phi1 := 2.0 * math.Pi * (1760.0*t - 1200.0*t*t/duration)
		phi2 := 2.0 * math.Pi * (1320.0*t - 900.0*t*t/duration)
		crystal := (0.55*math.Sin(phi1) + 0.45*math.Sin(phi2)) * math.Exp(-4.5*t)

		env := smoothEnvelope(t, duration, 0.010, 0.065)
		buf[i] = (0.75*crystal + 0.25*lpNoise*math.Exp(-4.0*t)) * env
	}
	return normalizeSamples(buf, 0.270)
}

// synthesizePointDefense produces three subtle, rapid TOS phaser micro-pulses (1180 Hz -> 880 Hz).
func synthesizePointDefense() []int16 {
	duration := 0.24 // 240 ms (3 micro-bursts of 80ms)
	numSamples := int(float64(sampleRate) * duration)
	buf := make([]float64, numSamples)

	burstDuration := 0.08
	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		burstIdx := int(t / burstDuration)
		burstT := t - float64(burstIdx)*burstDuration

		if burstT < 0.065 {
			bEnv := smoothEnvelope(burstT, 0.065, 0.005, 0.015)
			phi := 2.0*math.Pi*(1180.0*burstT-0.5*300.0*burstT*burstT/0.065) + 1.5*math.Sin(2.0*math.Pi*24.0*burstT)
			buf[i] = (0.78*math.Sin(phi) + 0.22*math.Sin(2.0*phi)) * bEnv
		}
	}
	return normalizeSamples(buf, 0.265)
}

// synthesizeCommChime produces the iconic 3-note Star Trek TOS bridge intercom bosun whistle:
// E6 (1318.5 Hz) -> A5 (880 Hz) -> E6 (1318.5 Hz) with delicate vibrato.
func synthesizeCommChime() []int16 {
	duration := 0.54 // 120ms + 130ms + 290ms = 540ms
	numSamples := int(float64(sampleRate) * duration)
	buf := make([]float64, numSamples)

	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		var freq, noteT, noteDur, decayRate float64
		if t < 0.120 {
			freq = 1318.51 // E6
			noteT = t
			noteDur = 0.120
			decayRate = 2.5
		} else if t < 0.250 {
			freq = 880.00 // A5
			noteT = t - 0.120
			noteDur = 0.130
			decayRate = 2.5
		} else {
			freq = 1318.51 // E6
			noteT = t - 0.250
			noteDur = 0.290
			decayRate = 3.0
		}

		phi := 2.0*math.Pi*freq*noteT + 0.5*math.Sin(2.0*math.Pi*6.5*noteT)
		env := smoothEnvelope(noteT, noteDur, 0.010, 0.030) * math.Exp(-decayRate*noteT)
		buf[i] = (0.84*math.Sin(phi) + 0.16*math.Sin(2.0*phi)) * env
	}
	return normalizeSamples(buf, 0.270)
}

// synthesizeComputerBeep produces a soft, delicate TOS library computer dual-tone crystal blip
// (1046.5 Hz C6 -> 1318.5 Hz E6 over 180ms).
func synthesizeComputerBeep() []int16 {
	duration := 0.18 // 180 ms
	numSamples := int(float64(sampleRate) * duration)
	buf := make([]float64, numSamples)

	split := 0.085
	for i := 0; i < numSamples; i++ {
		t := float64(i) / float64(sampleRate)
		var freq, noteT, noteDur float64
		if t < split {
			freq = 1046.50 // C6
			noteT = t
			noteDur = split
		} else {
			freq = 1318.51 // E6
			noteT = t - split
			noteDur = duration - split
		}
		env := smoothEnvelope(noteT, noteDur, 0.008, 0.020)
		phi := 2.0 * math.Pi * freq * noteT
		buf[i] = math.Sin(phi) * env
	}
	return normalizeSamples(buf, 0.250)
}


