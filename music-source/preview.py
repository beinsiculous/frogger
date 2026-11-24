#!/usr/bin/env python3
"""Render synthetic engine mixes, preserving authored short TSV inputs unchanged."""
import array
import math
from pathlib import Path
import subprocess
import wave

SOURCE = Path(__file__).resolve().parent
ENGINE = SOURCE.parents[2]/'insiculous_2d'
PREVIEWS = SOURCE/'previews'
ACTIONS = ['hop','hop_two','death','home','clear','menu']


def render(tool, recipe, seconds, name):
    with (PREVIEWS/f'{name}.log').open('w') as log:
        subprocess.run([str(tool),str(SOURCE.parent/'assets'),'ai/music/ai_chicken_coop.mid',
                        str(recipe),str(seconds),str(PREVIEWS/f'{name}.wav')],
                       stdout=log,stderr=subprocess.STDOUT,check=True)
    report = (PREVIEWS/f'{name}.log').read_text()
    for counter in ['clipped','limited_samples']:
        if f'{counter}=0 ' not in report:
            raise RuntimeError(f'{name}: {counter} must be zero')
    for counter in ['non_finite_samples','commands_rejected','rejected_triggers',
                    'late_triggers','stale_triggers','silent_triggers']:
        if f'{counter}: 0,' not in report:
            raise RuntimeError(f'{name}: {counter} must be zero')


def read_audio(name):
    with wave.open(str(PREVIEWS/f'{name}.wav'),'rb') as recording:
        return recording.getparams(),array.array('h',recording.readframes(recording.getnframes()))


def rms(samples):
    return math.sqrt(sum((sample/32768)**2 for sample in samples)/len(samples))


def main():
    PREVIEWS.mkdir(exist_ok=True)
    dense = []
    for step in range(1020):
        second = step/20
        if step%100<70:
            dense.extend([(second,'hop',1),(second,'hop_two',2)])
        for period,action,source in [(47,'death',3),(73,'home',3),(211,'clear',3),(131,'menu',4)]:
            if step%period==0:
                dense.append((second,action,source))
    dense_path = PREVIEWS/'dense.tsv'
    dense_path.write_text(''.join(f'{seconds:.3f} {action} {source}\n' for seconds,action,source in dense))
    subprocess.run(['cargo','build','--manifest-path',str(ENGINE/'Cargo.toml'),'-p','music_midi','--example','audition'],check=True)
    tool = ENGINE/'target/debug/examples/audition'
    for name in ['idle','sparse','dense']:
        render(tool,dense_path if name=='dense' else SOURCE/f'{name}.tsv',52,name)
    render(tool,SOURCE/'idle.tsv',8,'demo-idle')
    parameters,idle = read_audio('demo-idle')
    for action in ACTIONS:
        recipe = SOURCE/f'{action}.tsv'
        event_times = [float(line.split()[0]) for line in recipe.read_text().splitlines()
                       if line.strip() and not line.lstrip().startswith('#')]
        if not event_times or any(not math.isfinite(second) or not 0 <= second <= 7.9 for second in event_times):
            raise ValueError(f'{action}: demo events must be within 0–7.9 seconds for full onset windows')
        render(tool,recipe,8,f'demo-{action}')
        mixed_parameters,mixed = read_audio(f'demo-{action}')
        if mixed_parameters != parameters or len(mixed) != len(idle):
            raise ValueError(f'{action}: action and idle PCM formats/lengths differ')
        # The identical idle subtraction is linear only when both limiters stayed off.
        isolated = array.array('h',(sample-background for sample,background in zip(mixed,idle)))
        peak = max(abs(sample) for sample in isolated)/32768
        if not 0<rms(isolated)<peak<0.8913:
            raise RuntimeError(f'{action}: invalid isolated signal level')
        with wave.open(str(PREVIEWS/f'isolated-{action}.wav'),'wb') as recording:
            recording.setparams(parameters)
            recording.writeframes(isolated.tobytes())
        cue_windows,backing_windows = [],[]
        for second in event_times:
            start = int(second*parameters.framerate)*2
            end = start+int(0.1*parameters.framerate)*2
            cue_windows.extend(isolated[start:end])
            backing_windows.extend(idle[start:end])
        cue_level,backing_level = rms(cue_windows),rms(backing_windows)
        (PREVIEWS/f'isolated-{action}.log').write_text(
            f'peak={peak:.6f} rms={rms(isolated):.6f} onset_rms={cue_level:.6f} '
            f'backing_onset_rms={backing_level:.6f} cue_to_backing_db={20*math.log10(cue_level/backing_level):.2f} '
            'method=engine_mix_minus_identical_idle limiter_inactive=true\n')


if __name__=='__main__':
    main()
