#!/usr/bin/env python3
"""Author Crosswalk Carnival; running this replaces hand-edited project and patch sources."""
import json
from pathlib import Path

SOURCE = Path(__file__).resolve().parent


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def note(key, tick, duration=90, velocity=96):
    return dict(pitch=key, start_tick=tick, duration_ticks=duration, velocity=velocity)


def main():
    specifications = [
        ('offbeat_bass',58,'saw',0.27,0.17,True),
        ('carnival_lead',0,'square',0.18,0.16,True),
        ('ska_chord',1,'saw',0.14,0.075,True),
        ('crosswalk_kick',0,'sine',0.80,0.13,False),
        ('crosswalk_snare',0,'noise',0.42,0.085,False),
        ('swing_hat',0,'noise',0.14,0.024,False),
        ('round_hop',56,'triangle',0.65,0.20,True),
        ('tall_hop',72,'sine',0.68,0.105,True),
        ('coop_crash',57,'saw',0.95,0.26,True),
        ('carnival_tom',47,'sine',0.26,0.11,True),
    ]
    instruments=[]
    for index,(name,program,waveform,gain,decay,pitched) in enumerate(specifications):
        frequency=220.0 if name=='coop_crash' else 440.0
        patch=dict(name=name,sample_rate=32000,seed=11600+index,gain=gain,
                   duration_seconds=1.0,pitch_tracking=pitched,
                   oscillators=[dict(waveform=waveform,start_frequency_hz=frequency if pitched else 170.0,
                                     end_frequency_hz=frequency if pitched else 43.0,amplitude=1.0)],
                   envelope=dict(attack_seconds=0.001,decay_seconds=decay,sustain_level=0.0,release_seconds=0.05))
        if name in ('offbeat_bass','coop_crash','ska_chord'):
            patch['filter']=dict(cutoff_hz={'offbeat_bass':650.0,'coop_crash':1100.0,'ska_chord':2000.0}[name],
                                 envelope_amount=1.3,resonance=0.18,stages=2)
        if name=='round_hop':
            patch['oscillators'].append(dict(waveform='sine',start_frequency_hz=1760.0,end_frequency_hz=1760.0,amplitude=0.22))
        if name=='tall_hop':
            patch['oscillators'].append(dict(waveform='triangle',start_frequency_hz=880.0,end_frequency_hz=880.0,amplitude=0.40))
        if name in ('crosswalk_snare','swing_hat'):
            patch['filter']=dict(mode='high_pass',cutoff_hz=1100.0 if name=='crosswalk_snare' else 6800.0,stages=1)
        save(SOURCE/'patches'/f'{name}.json',patch)
        instruments.append(dict(name=name,source=dict(type='synth',patch=f'patches/{name}.json'),midi_program=program,gain=1.0,pan=0.0))
    tracks=[dict(name=name,instrument=instrument,notes=[]) for name,instrument in
            [('Bounce',0),('Crosswalk',1),('Upstroke',2),('Kick',3),('Snare',4),('Hat',5),('Tom',9),
             ('galop',6),('squawk',8),('fanfare',6),('cadence',6),('jam',7)]]
    # Long-short swung pairs and descending calls answer across two-beat bars.
    hooks=[
        [(0,74,200),(320,71,100),(480,67,280),(800,71,120)],
        [(0,76,280),(320,74,120),(640,71,240)],
        [(0,79,280),(320,76,120),(480,74,120),(800,71,120)],
        [(0,67,120),(320,71,100),(480,74,360)],
        [(0,83,280),(320,79,100),(480,76,280),(800,74,120)],
        [(160,79,240),(480,83,120),(800,86,120)],
        [(0,88,280),(320,86,120),(640,83,240)],
        [(0,79,120),(320,76,100),(480,71,360)],
    ]
    for bar in range(32):
        start=bar*960
        reduced=16<=bar<24
        for offset,key,length in [(0,43,100),(320,50,110),(480,47,100),(800,52,110)]:
            if not reduced or offset in (0,800):
                tracks[0]['notes'].append(note(key,start+offset,length,108 if offset==0 else 92))
        if not reduced or bar%2==0:
            for offset,key,length in hooks[bar%4+(4 if 8<=bar<16 else 0)]:
                tracks[1]['notes'].append(note(key,start+offset,length,90 if bar>=24 else 80))
        for offset in ([800] if reduced else [320,800]):
            for key in ([55,59,62] if bar%2==0 else [55,62,64]):
                tracks[2]['notes'].append(note(key,start+offset,100,75))
        for offset in ([0] if reduced else [0,640]):
            tracks[3]['notes'].append(note(36,start+offset,85,118 if offset==0 else 85))
        tracks[4]['notes'].append(note(38,start+480,80,113))
        for offset in ([320,800] if reduced else [0,320,480,800]):
            tracks[5]['notes'].append(note(42,start+offset,35,76 if offset in (320,800) else 45))
        if bar%8==7:
            for offset,key in [(600,50),(720,47),(840,43)]:
                tracks[6]['notes'].append(note(key,start+offset,70,95))
            tracks[4]['notes'].append(note(38,start+900,40,90))
    tracks[0]['notes'].append(note(43,30600,120,112))
    for index,key in enumerate([74,71,67,76,74,71,79,67]):
        tracks[7]['notes'].append(note(key,index*240,70,82))
    for key in [43,50]:
        tracks[8]['notes'].append(note(key,0,360,116))
    for key in [79,83,86]:
        tracks[9]['notes'].append(note(key,0,360,120))
    for key in [79,83,86,88]:
        tracks[10]['notes'].append(note(key,0,480,124))
    for index,key in enumerate([71,76,74,79]):
        tracks[11]['notes'].append(note(key,index*240,100,95))
    project=dict(format='midi-evil-project',version=2,name='Crosswalk Carnival',tempo_bpm=152,
                 beats_per_bar=2,beat_unit=4,ticks_per_quarter=480,sample_rate=32000,
                 master_gain=0.8,instruments=instruments,tracks=tracks)
    (SOURCE/'crosswalk_carnival.evil').write_text(json.dumps(project,separators=(',',':'))+'\n')
    presets,renders=[],[]
    for index,(name,program,_,_,_,pitched) in enumerate(specifications):
        keys={entry['pitch'] for track in tracks if track['instrument']==index for entry in track['notes']}
        if name=='tall_hop':
            keys.update(entry['pitch'] for entry in tracks[7]['notes'])
        zones=[]
        for key in sorted(keys):
            filename=f'rendered/{name}_{key}.wav'
            renders.append(dict(instrument=index,key=key,ticks=720,wav=filename))
            zones.append(dict(wav=filename,keys=[key,key],root=key,release_seconds=0.05,longest_note_seconds=0.40))
        if pitched:
            presets.append(dict(name=name,bank=0,program=program,zones=zones))
        else:
            percussion=next((preset for preset in presets if preset['bank']==128),None)
            if percussion is None:
                percussion=dict(name='Crosswalk drums',bank=128,program=0,zones=[])
                presets.append(percussion)
            percussion['zones'].extend(zones)
    save(SOURCE/'bank.json',dict(version=1,name='Crosswalk Carnival',copyright='Original AI synthesis; Be Insiculous; CC0-1.0',presets=presets))
    save(SOURCE/'render.json',dict(seed=11600,sample_rate=32000,velocity=127,gain=0.8,
                                  backing_pitch_classes=[2,4,7,11],cue_pitch_classes=[2,4,7,11],renders=renders))


if __name__=='__main__':
    main()
