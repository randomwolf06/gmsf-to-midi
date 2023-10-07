# gmsf-to-midi
A tool to convert .GMSF (Growtopia Music Simulator Final) files to .midi

# Compilation
Install Rust and run : 
```cargo build --release``` 

# Usage
```bash
gmsf-to-midi [FILES...]
```

## config.json format
```javascript
{
  
  "midi-channel-map": {
    // NOTE: midi channel id is limited from 0-15. Drums will always occupy channel 9
    "(midi channel id)" : { "patch" : 0,  "name" : "Stupid Channel Name"},
  },
  
  "gmsf-note-map" : {
    // Accidentals: "Natural", "Sharp", "Flat".
    // "Note" format - { "Note" : [(midi channel id), "(accidental)"] },
    "(GMSF note id)" : { "Note" : [0, "Natural"] },
    "(GMSF note id)" : { "Note" : [0, "Sharp"] },
    "(GMSF note id)" : { "Note" : [0, "Flat"] },
    // "LowNote" format - Same as "Note", 1 octave lower
    "(GMSF note id)" : { "LowNote" : [0, "Natural"] },
    "(GMSF note id)" : { "LowNote" : [0, "Sharp"] },
    "(GMSF note id)" : { "LowNote" : [0, "Flat"] },
    // "HighNote" format - Same as "Note", 1 octave higher
    "(GMSF note id)" : { "HighNote" : [0, "Natural"] },
    "(GMSF note id)" : { "HighNote" : [0, "Sharp"] },
    "(GMSF note id)" : { "HighNote" : [0, "Flat"] },
    // Special
    "(GMSF note id)" : "Drums",
    "(GMSF note id)" : "RepeatBegin",
    "(GMSF note id)" : "RepeatEnd",
    "(GMSF note id)" : "Other", // Placeholder, for unused/unimplemented stuff
    
  }
}

```
