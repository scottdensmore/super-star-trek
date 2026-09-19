package audio

import (
	"bytes"
	"encoding/binary"
	"testing"
)

func TestSynthesizeWav_HeaderValid(t *testing.T) {
	sounds := []SoundID{
		SoundPhaser,
		SoundTorpedoLaunch,
		SoundExplosion,
		SoundRedAlert,
		SoundDock,
		SoundWarp,
		SoundDamage,
		SoundShields,
		SoundVictory,
		SoundDefeat,
	}

	for _, s := range sounds {
		t.Run(string(s), func(t *testing.T) {
			wav := SynthesizeWav(s)
			if len(wav) < 44 {
				t.Fatalf("WAV buffer too small: %d bytes", len(wav))
			}
			if string(wav[0:4]) != "RIFF" {
				t.Errorf("expected RIFF header, got %s", string(wav[0:4]))
			}
			riffSize := binary.LittleEndian.Uint32(wav[4:8])
			if int(riffSize) != len(wav)-8 {
				t.Errorf("RIFF size mismatch: header %d, actual %d", riffSize, len(wav)-8)
			}
			if string(wav[8:12]) != "WAVE" {
				t.Errorf("expected WAVE format, got %s", string(wav[8:12]))
			}
			if string(wav[12:16]) != "fmt " {
				t.Errorf("expected fmt subchunk, got %s", string(wav[12:16]))
			}
			subchunk1Size := binary.LittleEndian.Uint32(wav[16:20])
			if subchunk1Size != 16 {
				t.Errorf("expected subchunk1 size 16, got %d", subchunk1Size)
			}
			audioFormat := binary.LittleEndian.Uint16(wav[20:22])
			if audioFormat != 1 {
				t.Errorf("expected PCM format (1), got %d", audioFormat)
			}
			numChannels := binary.LittleEndian.Uint16(wav[22:24])
			if numChannels != 1 {
				t.Errorf("expected mono (1), got %d", numChannels)
			}
			sampleRate := binary.LittleEndian.Uint32(wav[24:28])
			if sampleRate != 8000 {
				t.Errorf("expected 8000 Hz, got %d", sampleRate)
			}
			byteRate := binary.LittleEndian.Uint32(wav[28:32])
			if byteRate != 16000 {
				t.Errorf("expected byte rate 16000, got %d", byteRate)
			}
			blockAlign := binary.LittleEndian.Uint16(wav[32:34])
			if blockAlign != 2 {
				t.Errorf("expected block align 2, got %d", blockAlign)
			}
			bitsPerSample := binary.LittleEndian.Uint16(wav[34:36])
			if bitsPerSample != 16 {
				t.Errorf("expected 16 bits per sample, got %d", bitsPerSample)
			}
			if string(wav[36:40]) != "data" {
				t.Errorf("expected data subchunk, got %s", string(wav[36:40]))
			}
			dataLen := binary.LittleEndian.Uint32(wav[40:44])
			if int(dataLen) != len(wav)-44 {
				t.Errorf("data length mismatch: header %d, actual %d", dataLen, len(wav)-44)
			}
			if dataLen == 0 {
				t.Errorf("expected audio data samples, got 0 bytes")
			}
		})
	}
}

func TestSynthesizeWav_Deterministic(t *testing.T) {
	s1 := SynthesizeWav(SoundPhaser)
	s2 := SynthesizeWav(SoundPhaser)
	if !bytes.Equal(s1, s2) {
		t.Errorf("SynthesizeWav output should be deterministic")
	}
}

func TestSynthesizeWav_UnknownSoundID(t *testing.T) {
	wav := SynthesizeWav(SoundID("unknown_sound"))
	if len(wav) < 44 {
		t.Fatalf("WAV buffer for unknown sound too small: %d bytes", len(wav))
	}
	if string(wav[0:4]) != "RIFF" {
		t.Errorf("expected RIFF header for unknown sound")
	}
}
