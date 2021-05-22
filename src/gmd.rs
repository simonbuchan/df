use crate::common::read_buf;
use std::{thread, time};

pub fn play_in_thread(data: Vec<u8>) -> impl Drop {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    struct Stop(Arc<AtomicBool>);
    impl Drop for Stop {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    let stop = Stop(Arc::new(AtomicBool::new(false)));

    thread::Builder::new()
        .name("midi_playback".to_string())
        .stack_size(0x1000)
        .spawn({
            let stop = stop.0.clone();
            move || {
                midi(data, &stop);
            }
        })
        .unwrap();

    stop
}

pub fn midi(data: Vec<u8>, stop: &std::sync::atomic::AtomicBool) {
    if !data.starts_with(b"MIDI") {
        println!("no GMD 'MIDI' signature");
        return;
    }
    let mut data = &data[8..];
    while !data.starts_with(b"MThd") {
        read_buf(&mut data, [0u8; 4]).unwrap();
        let len = u32::from_be_bytes(read_buf(&mut data, [0u8; 4]).unwrap());
        data = &data[len as usize..];
    }

    use bindings::{
        Result, RuntimeType,
        Windows::{Devices::Midi, Foundation::IAsyncOperation},
    };

    fn get<T: RuntimeType>(result: Result<IAsyncOperation<T>>) -> T {
        result.unwrap().get().unwrap()
    }

    let midi = get(Midi::MidiSynthesizer::CreateAsync());

    let smf = midly::Smf::parse(&data).unwrap();

    let mut events = match smf.header.format {
        midly::Format::SingleTrack | midly::Format::Parallel => smf
            .tracks
            .iter()
            .flat_map(|msgs| {
                msgs.iter().scan(0, |ticks, event| {
                    *ticks += event.delta.as_int();
                    Some((*ticks, event.kind))
                })
            })
            .collect::<Vec<_>>(),
        midly::Format::Sequential => smf
            .tracks
            .iter()
            .flatten()
            .scan(0, |ticks, event| {
                *ticks += event.delta.as_int();
                Some((*ticks, event.kind))
            })
            .collect::<Vec<_>>(),
    };

    events.sort_by_key(|(ts, _)| *ts);

    let mut elapsed_ticks = 0;
    let mut last_time = time::Instant::now();
    let mut beat = time::Duration::from_secs(1);

    for (ts_ticks, event) in events {
        if ts_ticks > elapsed_ticks {
            let ticks = ts_ticks - elapsed_ticks;
            let duration = match smf.header.timing {
                midly::Timing::Metrical(ticks_per_beat) => {
                    beat * ticks / ticks_per_beat.as_int() as u32
                }
                midly::Timing::Timecode(fps, subframe) => {
                    time::Duration::from_secs_f32(ticks as f32 / fps.as_f32() / subframe as f32)
                }
            };
            last_time += duration;
            elapsed_ticks = ts_ticks;

            // Semi-accurately wait. sleep() alone gives silly results!
            const SLEEP_ACCURACY_ESTIMATE: time::Duration = time::Duration::from_millis(1);
            while let Some(remaining) = last_time.checked_duration_since(time::Instant::now()) {
                if let Some(sleep_duration) = remaining.checked_sub(SLEEP_ACCURACY_ESTIMATE) {
                    thread::sleep(sleep_duration);
                } else {
                    thread::yield_now();
                }
            }
        }

        if stop.load(std::sync::atomic::Ordering::SeqCst) {
            println!("Cancelled");
            return;
        }

        match event {
            midly::TrackEventKind::Meta(event) => match event {
                midly::MetaMessage::Tempo(us_per_beat) => {
                    beat = time::Duration::from_micros(us_per_beat.as_int().into());
                }
                _ => {}
            },
            midly::TrackEventKind::Midi { channel, message } => match message {
                midly::MidiMessage::NoteOff { key, vel } => {
                    midi.SendMessage(
                        Midi::MidiNoteOffMessage::CreateMidiNoteOffMessage(
                            channel.into(),
                            key.into(),
                            vel.into(),
                        )
                        .unwrap(),
                    )
                    .unwrap();
                }
                midly::MidiMessage::NoteOn { key, vel } => {
                    midi.SendMessage(
                        Midi::MidiNoteOnMessage::CreateMidiNoteOnMessage(
                            channel.into(),
                            key.into(),
                            vel.into(),
                        )
                        .unwrap(),
                    )
                    .unwrap();
                }
                midly::MidiMessage::Aftertouch { vel, key } => {
                    Midi::MidiPolyphonicKeyPressureMessage::CreateMidiPolyphonicKeyPressureMessage(
                        channel.into(),
                        key.into(),
                        vel.into(),
                    )
                    .unwrap();
                }
                midly::MidiMessage::Controller { controller, value } => {
                    midi.SendMessage(
                        Midi::MidiControlChangeMessage::CreateMidiControlChangeMessage(
                            channel.into(),
                            controller.into(),
                            value.into(),
                        )
                        .unwrap(),
                    )
                    .unwrap();
                }
                midly::MidiMessage::ProgramChange { program } => {
                    midi.SendMessage(
                        Midi::MidiProgramChangeMessage::CreateMidiProgramChangeMessage(
                            channel.into(),
                            program.into(),
                        )
                        .unwrap(),
                    )
                    .unwrap();
                }
                midly::MidiMessage::ChannelAftertouch { vel } => {
                    Midi::MidiChannelPressureMessage::CreateMidiChannelPressureMessage(
                        channel.into(),
                        vel.into(),
                    )
                    .unwrap();
                }
                midly::MidiMessage::PitchBend { bend } => {
                    midi.SendMessage(
                        Midi::MidiPitchBendChangeMessage::CreateMidiPitchBendChangeMessage(
                            channel.into(),
                            bend.0.as_int(),
                        )
                        .unwrap(),
                    )
                    .unwrap();
                }
            },
            midly::TrackEventKind::SysEx(bytes) => {
                if let Some(bytes) = bytes.strip_prefix(&[125]) {
                    let (kind, content) =
                        bytes.strip_suffix(&[0xF7]).unwrap().split_first().unwrap();
                    match kind {
                        3 => {
                            let content = content.strip_suffix(&[0]).unwrap();
                            println!("iMUSE: label: {:?}", std::str::from_utf8(content));
                        }
                        _ => {
                            println!("iMUSE: {}: {:?}", kind, content);
                        }
                    }
                } else {
                    println!("MIDI: SysEx: {:?}", bytes);
                }
            }
            midly::TrackEventKind::Escape(bytes) => {
                println!("MIDI: Esc: {:?}", bytes);
                // midi.SendBuffer(create_buffer(bytes).unwrap()).unwrap();
            }
        }
    }

    println!("MIDI: done");
}
