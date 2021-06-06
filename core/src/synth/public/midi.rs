use crate::soundfont::SoundFont;
use crate::synth::Synth;
use crate::utils::TypedIndex;

impl Synth {
    /**
    Send a noteon message.
     */
    pub fn noteon(&mut self, midi_chan: usize, key: u8, vel: u8) -> Result<(), &str> {
        if key >= 128 {
            log::error!("Key out of range");
            Err("Key out of range")
        } else if vel >= 128 {
            log::error!("Velocity out of range");
            Err("Velocity out of range")
        } else if let Some(channel) = self.channels.get_mut(midi_chan as usize) {
            if vel == 0 {
                self.noteoff(midi_chan, key);
                Ok(())
            } else if channel.preset().is_none() {
                log::warn!(
                    "noteon\t{}\t{}\t{}\t{}\t{}\t\t{}\t{}\t{}",
                    midi_chan,
                    key,
                    vel,
                    0,
                    (self.ticks as f32 / 44100.0f32),
                    0.0f32,
                    0,
                    "channel has no preset"
                );
                Err("Channel has no preset")
            } else {
                self.voices.release_voice_on_same_note(
                    &self.channels[midi_chan],
                    key,
                    self.min_note_length_ticks,
                );

                self.voices.noteid_add();

                self.sf_noteon(midi_chan, key, vel);
                Ok(())
            }
        } else {
            log::error!("Channel out of range");
            Err("Channel out of range")
        }
    }

    /**
    Send a noteoff message.
     */
    pub fn noteoff(&mut self, chan: usize, key: u8) {
        self.voices
            .noteoff(&self.channels[chan], self.min_note_length_ticks, key)
    }

    /**
    Send a control change message.
     */
    pub fn cc(&mut self, chan: usize, num: u16, val: u16) -> Result<(), ()> {
        if chan as usize >= self.channels.len() {
            log::warn!("Channel out of range",);
            return Err(());
        }
        if num >= 128 {
            log::warn!("Ctrl out of range",);
            return Err(());
        }
        if val >= 128 {
            log::warn!("Value out of range",);
            return Err(());
        }

        log::trace!("cc\t{}\t{}\t{}", chan, num, val);

        self.channel_cc(chan, num, val);

        Ok(())
    }

    /**
    Get a control value.
     */
    pub fn get_cc(&self, chan: usize, num: u16) -> Result<u8, &str> {
        if let Some(channel) = self.channels.get(chan) {
            if num >= 128 {
                log::warn!("Ctrl out of range");
                Err("Ctrl out of range")
            } else {
                let pval = channel.cc(num as usize);
                Ok(pval)
            }
        } else {
            log::warn!("Channel out of range",);
            Err("Channel out of range")
        }
    }

    pub fn all_notes_off(&mut self, chan: usize) {
        self.voices
            .all_notes_off(&self.channels, self.min_note_length_ticks, chan)
    }

    pub fn all_sounds_off(&mut self, chan: usize) {
        self.voices.all_sounds_off(chan)
    }

    /**
    Send a pitch bend message.
     */
    pub fn pitch_bend(&mut self, chan: usize, val: u16) -> Result<(), &str> {
        if let Some(channel) = self.channels.get_mut(chan) {
            log::trace!("pitchb\t{}\t{}", chan, val);

            const FLUID_MOD_PITCHWHEEL: u16 = 14;

            channel.set_pitch_bend(val as i16);

            self.voices
                .modulate_voices(&*channel, 0, FLUID_MOD_PITCHWHEEL);

            Ok(())
        } else {
            log::error!("Channel out of range",);
            Err("Channel out of range")
        }
    }

    /**
    Get the pitch bend value.
     */
    pub fn get_pitch_bend(&self, chan: usize) -> Result<i16, &str> {
        if let Some(channel) = self.channels.get(chan) {
            let pitch_bend = channel.pitch_bend();
            Ok(pitch_bend)
        } else {
            log::warn!("Channel out of range",);
            Err("Channel out of range")
        }
    }

    /**
    Set the pitch wheel sensitivity.
     */
    pub fn pitch_wheel_sens(&mut self, chan: usize, val: u16) -> Result<(), &str> {
        if let Some(channel) = self.channels.get_mut(chan) {
            log::trace!("pitchsens\t{}\t{}", chan, val);

            const FLUID_MOD_PITCHWHEELSENS: u16 = 16;

            channel.set_pitch_wheel_sensitivity(val);

            self.voices
                .modulate_voices(&*channel, 0, FLUID_MOD_PITCHWHEELSENS);

            Ok(())
        } else {
            log::error!("Channel out of range",);
            Err("Channel out of range")
        }
    }

    /**
    Get the pitch wheel sensitivity.
     */
    pub fn get_pitch_wheel_sens(&self, chan: usize) -> Result<u32, &str> {
        if let Some(channel) = self.channels.get(chan) {
            Ok(channel.pitch_wheel_sensitivity() as u32)
        } else {
            log::warn!("Channel out of range",);
            Err("Channel out of range")
        }
    }

    /**
    Send a program change message.
     */
    pub fn program_change(&mut self, chan: usize, prognum: u8) -> Result<(), ()> {
        if prognum >= 128 || chan >= self.channels.len() {
            log::error!("Index out of range (chan={}, prog={})", chan, prognum);
            return Err(());
        }

        let banknum = self.channels[chan as usize].banknum();
        self.channels[chan as usize].set_prognum(prognum);

        log::trace!("prog\t{}\t{}\t{}", chan, banknum, prognum);

        let mut preset =
            if self.channels[chan as usize].id() == 9 && self.settings.drums_channel_active {
                self.find_preset(128, prognum)
            } else {
                self.find_preset(banknum, prognum)
            };

        if preset.is_none() {
            let mut subst_bank = banknum as i32;
            let mut subst_prog = prognum;
            if banknum != 128 {
                subst_bank = 0;
                preset = self.find_preset(0, prognum);
                if preset.is_none() && prognum != 0 {
                    preset = self.find_preset(0, 0);
                    subst_prog = 0;
                }
            } else {
                preset = self.find_preset(128, 0);
                subst_prog = 0;
            }
            if preset.is_none() {
                log::warn!(
                        "Instrument not found on channel {} [bank={} prog={}], substituted [bank={} prog={}]",
                        chan, banknum, prognum,
                        subst_bank, subst_prog);
            }
        }

        self.channels[chan as usize].set_sfontnum(preset.as_ref().map(|p| p.0));
        self.channels[chan as usize].set_preset(preset.map(|p| p.1.clone()));

        Ok(())
    }

    /**
    Set channel pressure
     */
    pub fn channel_pressure(&mut self, chan: usize, val: u16) -> Result<(), &str> {
        if let Some(channel) = self.channels.get_mut(chan as usize) {
            log::trace!("channelpressure\t{}\t{}", chan, val);

            const FLUID_MOD_CHANNELPRESSURE: u16 = 13;
            channel.set_channel_pressure(val as i16);

            self.voices
                .modulate_voices(&self.channels[chan], 0, FLUID_MOD_CHANNELPRESSURE);
            Ok(())
        } else {
            log::error!("Channel out of range",);
            Err("Channel out of range")
        }
    }

    /**
    Set key pressure (aftertouch)
     */
    pub fn key_pressure(&mut self, chan: usize, key: u8, val: u8) -> Result<(), ()> {
        if key > 127 {
            return Err(());
        }
        if val > 127 {
            return Err(());
        }

        log::trace!("keypressure\t{}\t{}\t{}", chan, key, val);

        if let Some(channel) = self.channels.get_mut(chan) {
            channel.set_key_pressure(key as usize, val as i8);

            self.voices.key_pressure(&self.channels[chan], key);
            Ok(())
        } else {
            log::error!("Channel out of range",);
            Err(())
        }
    }

    /**
    Select a bank.
     */
    pub fn bank_select(&mut self, chan: u8, bank: u32) -> Result<(), &str> {
        if let Some(channel) = self.channels.get_mut(chan as usize) {
            channel.set_banknum(bank);
            Ok(())
        } else {
            log::error!("Channel out of range",);
            Err("Channel out of range")
        }
    }

    /**
    Select a sfont.
     */
    pub fn sfont_select(&mut self, chan: u8, sfont_id: TypedIndex<SoundFont>) -> Result<(), &str> {
        if let Some(channel) = self.channels.get_mut(chan as usize) {
            channel.set_sfontnum(Some(sfont_id));
            Ok(())
        } else {
            log::error!("Channel out of range",);
            Err("Channel out of range")
        }
    }

    /**
    Select a preset for a channel. The preset is specified by the
    SoundFont ID, the bank number, and the preset number. This
    allows any preset to be selected and circumvents preset masking
    due to previously loaded SoundFonts on the SoundFont stack.
     */
    pub fn program_select(
        &mut self,
        chan: u8,
        sfont_id: TypedIndex<SoundFont>,
        bank_num: u32,
        preset_num: u8,
    ) -> Result<(), &str> {
        let preset = self.get_preset(sfont_id, bank_num, preset_num);

        if let Some(channel) = self.channels.get_mut(chan as usize) {
            if preset.is_none() {
                log::error!(
                    "There is no preset with bank number {} and preset number {} in SoundFont {:?}",
                    bank_num,
                    preset_num,
                    sfont_id
                );
                Err("This preset does not exist")
            } else {
                channel.set_sfontnum(Some(sfont_id));
                channel.set_banknum(bank_num);
                channel.set_prognum(preset_num);
                channel.set_preset(preset);
                Ok(())
            }
        } else {
            log::error!("Channel out of range",);
            Err("Channel out of range")
        }
    }

    /**
    Returns the program, bank, and SoundFont number of the preset on a given channel.
     */
    pub fn get_program(&self, chan: u8) -> Result<(Option<TypedIndex<SoundFont>>, u32, u32), &str> {
        if let Some(channel) = self.channels.get(chan as usize) {
            Ok((
                channel.sfontnum(),
                channel.banknum(),
                channel.prognum() as u32,
            ))
        } else {
            log::warn!("Channel out of range",);
            Err("Channel out of range")
        }
    }

    /**
    Send a bank select and a program change to every channel to reinitialize the preset of the channel.

    This function is useful mainly after a SoundFont has been loaded, unloaded or reloaded.
     */
    pub fn program_reset(&mut self) {
        for id in 0..self.channels.len() {
            let preset = self.channels[id].prognum();
            self.program_change(id, preset).ok();
        }
    }

    /**
    Send a reset.

    A reset turns all the notes off and resets the controller values.

    Purpose:
    Respond to the MIDI command 'system reset' (0xFF, big red 'panic' button)
     */
    pub fn system_reset(&mut self) {
        self.voices.system_reset();

        let preset = self.find_preset(0, 0).map(|p| p.1);
        for channel in self.channels.iter_mut() {
            channel.init(preset.clone());
            channel.init_ctrl(0);
        }

        self.chorus.reset();
        self.reverb.reset();
    }
}
