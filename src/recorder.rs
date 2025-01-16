pub enum Codec {
    WebM,
    H264,
    TGA,
    WAV,
}

impl Codec {
    pub fn params(&self) -> String{
        match self {
            Codec::WebM => "webm".to_owned(),
            Codec::H264 => "h264".to_owned(),
            Codec::TGA => "raw".to_owned(),
            Codec::WAV => "wav".to_owned(),
        }
    }
}

// BIG TODO

/*
Idea: 

Have buttons to set In/Out Markers on the timeline
Button "Record"
Opens up Window with settings
+ Framerate
+ Codec
+ Save location
+ Postprocessing with ffmpeg?

Button to start recording
Sends disconnect, playdemo to tf2, skips to in point and pauses, sets end tick to out point, set framerate, startmovie, resume playback
name could be demoname + in/outpoint
when playback finishes endmovie, copy generated files to selected directory

if ffmpeg is selected run that command
*/