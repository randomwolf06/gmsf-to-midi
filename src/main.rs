use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::{fs::File, io::Write, io::BufWriter};
use std::io::{self, BufReader, Read};
use serde::{Serialize, Deserialize};
use std::{fs, env};

const GMSF_KEY_LOOKUP : [u8; 14] = [
    71,
    69,
    67,
    65,
    64,
    62,
    60,
    59,
    57,
    55,
    53,
    52,
    50,
    48,
];

const GMSF_DRUMS_LOOKUP : [u8; 14] = [
    36, // Bass Drum 1
    39, // Hand Clap
    59, // Ride Cymbal 3
    38, // Acoustic Snare 
    43, // Hi Floor Tom
    45, // Low Tom
    49, // Crash Cymbal
    36, // ditto... //
    39,
    59,
    38,
    43,
    45,
    49,
];


const DELTATIME : u32 = 96;
const DELTATIME_PER_BLOCK : u32 = DELTATIME/4;

fn var_len_from(mut value : u32) -> Vec<u8> {
    let mut bytes : Vec<u8> = vec![];
    let mut buf : u32;
    buf = value & 0x7f;
    value >>= 7;
    while value > 0 {
        buf <<= 8;
        buf |= 0x80;
        buf += value & 0x7f;
        value >>= 7;
    }
    loop {
        bytes.push(buf as u8);
        if buf & 0x80 != 0 {
            buf >>= 8;
        } else {
            break;
        }
    }
    return bytes;
}

enum MidiEventType {
    NoteOff(u8),
    NoteOn(u8),
    ProgramChange(u8),
}
enum MidiMetaEventType {
    TrackName(String),
    ChannelPrefix(u8),
    SetTempo(u32),
    EndOfTrack,
}

impl MidiEventType {
    fn as_vec(&self, delta : u32, channel_id : u8) -> Vec<u8> {
        match &self {
            MidiEventType::NoteOff(key) => {
                vec![var_len_from(delta), vec![0x80 | channel_id, *key, 0]].concat()
            },
            MidiEventType::NoteOn(key) => {
                vec![var_len_from(delta), vec![0x90 | channel_id, *key, 64]].concat()
            },
            MidiEventType::ProgramChange(patch) => {
                vec![var_len_from(delta), vec![0xC0 | channel_id, *patch]].concat()
            },
        }
    }
}
impl MidiMetaEventType {
    fn as_vec(&self, delta : u32) -> Vec<u8> {
        match &self {
            MidiMetaEventType::TrackName(name) => {
                vec![var_len_from(delta), vec![0xFF, 0x03, (name.len() as u8)], name.as_bytes().to_vec()].concat()
            }
            MidiMetaEventType::ChannelPrefix(channel) => {
                vec![var_len_from(delta), vec![0xFF, 0x20, 0x01, *channel]].concat()
            }
            MidiMetaEventType::SetTempo(bpm) => {
                vec![var_len_from(delta), vec![0xFF, 0x51, 0x03], (60_000_000 / bpm).to_be_bytes()[1..4].to_vec()].concat()
            }
            MidiMetaEventType::EndOfTrack => {
                vec![var_len_from(delta), vec![0xFF, 0x2F, 0x00]].concat()
            }
        }
    }
}


#[derive(Clone, Copy, PartialEq, Debug)]
#[derive(Serialize, Deserialize)]
enum GMSFSheetType {
    Note(u8, Accidental),
    LowNote(u8, Accidental),
    Drums,
    RepeatBegin,
    RepeatEnd,
    Other,
}
#[derive(Clone, Copy, PartialEq, Debug)]
#[derive(Serialize, Deserialize)]
enum Accidental {
    Natural,
    Flat,
    Sharp,
}
#[derive(Serialize, Deserialize, Debug)]
struct TrackInfo {
    patch : u8,
    name : String,

}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    midi_track_map : HashMap<u8, TrackInfo>,
    gmsf_sheet_map : HashMap<u8, GMSFSheetType>,
}

fn channel_and_key_from_gmsf_sheet(sheet_type : GMSFSheetType, y : usize) -> Option<(u8, u8)> {
    match sheet_type {
        GMSFSheetType::Note(channel_id, accidental) => {
            let mut key : u8 = GMSF_KEY_LOOKUP[y];
            if let Accidental::Flat = accidental  { key -= 1; }
            if let Accidental::Sharp = accidental { key += 1; }
            
            Some((channel_id, key))
        }
        GMSFSheetType::LowNote(channel_id, accidental) => {
            let mut key : u8 = GMSF_KEY_LOOKUP[y] - 24;
            if let Accidental::Flat = accidental  { key -= 1; }
            if let Accidental::Sharp = accidental { key += 1; }
            
            Some((channel_id, key))
        }
        GMSFSheetType::Drums => Some((9, GMSF_DRUMS_LOOKUP[y])),
        _ => None
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
struct RepeatEnd {
    start_pos : usize,
    use_counter : i32,
    max_use : i32,
}

fn convert_gmsf_to_midi(path : &str, config : &Config) -> io::Result<()> {

        //------- Import time!
    let mut infile : BufReader<File> = BufReader::new(File::open(path)?);

    let mut read_header : [u8; 4] = [0; 4];
    infile.read(&mut read_header)?;
    
    if read_header != *b"GMSF" {
        println!("Error! Invalid GMSF File!");
        return Err(io::Error::from(io::ErrorKind::InvalidData));
    }
    
    let mut buf8 : [u8; 1] = [0];
    let mut buf16 : [u8; 2] = [0; 2];
    
    infile.read(&mut buf8)?;
    let read_version = u8::from_le_bytes(buf8);
    infile.read(&mut buf8)?;
    let read_audiogear_id = u8::from_le_bytes(buf8);

    infile.read(&mut buf16)?;
    let read_bpm  = i16::from_le_bytes(buf16) as u32;
    infile.read(&mut buf16)?;
    let read_width = i16::from_le_bytes(buf16) as usize;
    infile.read(&mut buf16)?;
    let read_height = i16::from_le_bytes(buf16) as usize;
    
    let mut song_data : HashMap<u8, Vec<HashSet<u8>>> = HashMap::new();
    let mut repeat_ends : Vec<Vec<RepeatEnd>> = vec![Vec::new(); read_width];

    for y in 0..read_height {
        let mut repeat_begin_pos : Vec<usize> = vec![];
        for x in 0..read_width {
            infile.read(&mut buf8)?;
            let read_id = u8::from_le_bytes(buf8);
            if read_id == 0 { continue; }
            if read_id == read_audiogear_id {
                for _ in 0..5 {
                    infile.read(&mut buf8)?;
                    let inner_id = u8::from_le_bytes(buf8);
                    infile.read(&mut buf8)?;
                    let inner_y = u8::from_le_bytes(buf8) as usize;
                    if inner_id == 0 { continue; }
                    if let Some(sheet) = config.gmsf_sheet_map.get(&inner_id) {
                        if let Some((channel, key)) = channel_and_key_from_gmsf_sheet(*sheet, inner_y) {
                            song_data.entry(channel).or_insert(vec![HashSet::new(); read_width]);
                            song_data.entry(channel).and_modify(|v| { v[x].insert(key); });
                        }
                    }
                }
                infile.seek_relative(1)?; //Skip volume bytes for now

            } else if let Some(sheet) = config.gmsf_sheet_map.get(&read_id) {
        
                if let Some((channel, key)) = channel_and_key_from_gmsf_sheet(*sheet, y) {
                    song_data.entry(channel).or_insert(vec![HashSet::new(); read_width]);
                    song_data.entry(channel).and_modify(|v| { v[x].insert(key); });
                    
                }
                if *sheet == GMSFSheetType::RepeatBegin {
                    repeat_begin_pos.push(x);
                }
                if *sheet == GMSFSheetType::RepeatEnd {
                    let last_begin : usize;
                    if let Some(n) = repeat_begin_pos.last() {
                        last_begin= *n;
                    } else {
                        last_begin = 0;
                    }
                    repeat_ends[x].push(RepeatEnd { start_pos: last_begin, use_counter: 0, max_use: 1 })
                }
            }
        }
    }

    for v in repeat_ends.iter_mut() {
        v.sort();
        let mut combined : Vec<RepeatEnd> = vec![];
        let mut prev_repeat_end : Option<RepeatEnd> = None;
        for repeat_end in v.iter() {
            if let Some(prev) = prev_repeat_end  {
                if repeat_end.start_pos == prev.start_pos {
                    if let Some(last) = combined.last_mut() {
                        last.max_use += 1
                    }
                } else {
                    combined.push(RepeatEnd { start_pos: repeat_end.start_pos, use_counter: 0, max_use: 1 });
                }
            } else {
                combined.push(RepeatEnd { start_pos: repeat_end.start_pos, use_counter: 0, max_use: 1 });
            }
            prev_repeat_end = Some(*repeat_end);
        }
        *v = combined;
    }

    //---- Its converting time :Happy: -------------//

    let mut total_tracks : u16 = 1;

    let mut track_buffers : Vec<Vec<u8>> = vec![];
    
    let mut song_data = Vec::from_iter(song_data.iter());
    song_data.sort_by(|a, b| a.0.cmp(&b.0));

    for (instrument, track) in &song_data {
        let track_info;
        let channel_id;
        if let Some(n) = config.midi_track_map.get_key_value(&instrument) {
            track_info = n.1;
            channel_id = *n.0;
            if channel_id > 15 {
                println!("channel id should be 0-15, skipping track {}", channel_id);
                continue;
            }
        } else { continue; }

        total_tracks += 1;
        
        let mut track_buffer : Vec<u8> = vec![];
        
        let mut current_delta : u32 = 0;
        let mut last_delta : u32 = 0;
        let mut last_keys : Vec<u8> = vec![];

        track_buffer.append(&mut MidiMetaEventType::TrackName(track_info.name.clone()).as_vec(0));

        track_buffer.append(&mut MidiEventType::ProgramChange(track_info.patch).as_vec(0, channel_id));

        let mut x : usize = 0;

        while x < read_width {
            for last_key in last_keys.iter() {
                track_buffer.append(&mut MidiEventType::NoteOff(*last_key).as_vec(current_delta - last_delta, channel_id));
                last_delta = current_delta
            }
            if !last_keys.is_empty() {
                last_keys.clear();
            }
            for key in track[x].iter() {
                track_buffer.append(&mut MidiEventType::NoteOn(*key).as_vec(current_delta - last_delta, channel_id));
                last_delta = current_delta;
                last_keys.push(*key);
            }
            current_delta += DELTATIME_PER_BLOCK;
            let mut repeating : bool = false;
            for repeat_end in repeat_ends[x].iter_mut() {
                if repeat_end.use_counter >= repeat_end.max_use {
                    repeat_end.use_counter = 0;
                } else {
                    repeat_end.use_counter += 1;
                    x = repeat_end.start_pos;
                    repeating = true;
                    break;
                }
            }
            if !repeating { x += 1; }
        }
        track_buffer.append(&mut MidiMetaEventType::EndOfTrack.as_vec(0));
        track_buffers.push(track_buffer);
    }

    // export :)
    let mut filename = Path::new(path).file_name().unwrap().to_owned();
    filename.push(".mid");
    
    let mut outfile = BufWriter::new(File::create(filename)?);

    outfile.write(b"MThd")?;
    outfile.write(&(6 as u32).to_be_bytes())?;
    outfile.write(&(1 as u16).to_be_bytes())?; // Format
    outfile.write(&total_tracks.to_be_bytes())?;
    outfile.write(&(DELTATIME as u16).to_be_bytes())?;
    
    let header_track : Vec<u8> = vec![
        MidiMetaEventType::SetTempo(read_bpm).as_vec(0),
        MidiMetaEventType::EndOfTrack.as_vec(0)
    ].concat();

    outfile.write(b"MTrk")?;
    outfile.write(&(header_track.len() as u32).to_be_bytes())?;
    outfile.write(&header_track)?;

    for track_buffer in track_buffers {
        outfile.write(b"MTrk")?;
        outfile.write(&(track_buffer.len() as u32).to_be_bytes())?;
        outfile.write(&track_buffer)?;
    }

    Ok(())
}



fn main() {
    let config_json_reader = fs::read_to_string("config.json").unwrap_or_else(|e| 
        panic!("Cant open config.json! {}", e)
    );
    println!("{}", config_json_reader);
    let config : Config = serde_json::from_str(&config_json_reader).unwrap_or_else(|e| 
        panic!("Cant parse config.json! {}", e)
    );
    let mut args : VecDeque<String> = env::args().collect();
    args.pop_front();
    
    for filename in args {
        convert_gmsf_to_midi(&filename, &config).unwrap_or_else(|err| {
            println!("Oops! Something went wrong while converting {}, skipping... {}", filename , err);
        });
    }
}
