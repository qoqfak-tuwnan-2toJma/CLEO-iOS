use anyhow::Result;

/// Tells whether or not a script is running, and where the value came from.
pub enum State {
    /// A state automatically decided by CLEO.
    Auto(bool),

    /// A state selected by the user.
    User(bool),
}

impl State {
    pub fn value(self) -> bool {
        match self {
            State::Auto(v) | State::User(v) => v,
        }
    }
}

pub enum FocusWish {
    /// The script needs to retain the focus and execute its next instruction.
    RetainFocus,

    /// The system can move onto the next script; this script does not need the focus.
    MoveOn,
}

/// Information about a script that should be given to the user. Flags can be added both before and
/// while the script is running, so may represent either static or runtime information.
#[derive(PartialEq, Eq, Ord)]
pub enum Flag {
    /// The script is taking a long time to update, and may cause performance issues.
    Slow,

    /// The script uses instructions that aren't available in this version of CLEO.
    UsesUnimplemented,

    /// The script uses code that is specific to the Android platform/game.
    PlatformSpecific,

    /// The script has the same `identity` as another script.
    Duplicate(String),
}

impl Flag {
    /// Returns an integer value indicating how important this flag is. A *lower* value means that
    /// a flag is *more* important.
    fn importance(&self) -> u8 {
        match self {
            Flag::Slow => 3,
            Flag::UsesUnimplemented => 2,
            Flag::PlatformSpecific => 1,
            Flag::Duplicate(_) => 4,
        }
    }
}

impl PartialOrd for Flag {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let this_sev = self.importance();
        let other_sev = other.importance();

        if this_sev > other_sev {
            Some(std::cmp::Ordering::Greater)
        } else if other_sev < this_sev {
            Some(std::cmp::Ordering::Less)
        } else {
            // Importances are equal.
            None
        }
    }
}

impl std::fmt::Display for Flag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UsesUnimplemented => f.write_str("Requires features unavailable on iOS."),
            Self::PlatformSpecific => f.write_str("Contains some iOS-incompatible code."),
            Self::Duplicate(orig_name) => write!(f, "Duplicate of '{}'.", orig_name),
            Self::Slow => f.write_str("May be slowing down the game."),
        }
    }
}

pub type GameTime = u32;

/// An item that should be unique for a script's content and which can therefore be used to find
/// scripts that are identical.
#[derive(PartialEq)]
pub enum Identity {
    Scm(u64),
    Js(u64),
}

/// An entity that runs scripting code to affect the game state.
pub trait Script {
    /// Executes a single instruction from the script. Returns a `FocusWish` describing what the
    /// executing system should do next (continue with this script or move on).
    ///
    /// If something goes wrong during execution, this method **must** return an error. Errors
    /// during script execution have to be handled appropriately to avoid corrupting the game
    /// state.
    fn exec_single(&mut self) -> Result<FocusWish>;

    /// Executes a block of instructions. A "block" continues until `exec_single` returns
    /// `FocusWish::MoveOn` to indicate that the script no longer requires focus.
    ///
    /// If `exec_single` returns an error, this method will return that error immediately.
    ///
    /// Instructions are executed in blocks because some instructions must run consecutively (and
    /// without a gap in between) as they assume that the game state does not change from one
    /// instruction to the next.
    fn exec_block(&mut self) -> Result<()> {
        if !self.is_ready() {
            return Ok(());
        }

        // Record the time at which we start executing instructions so we have a reference point.
        let start_time = std::time::Instant::now();

        while let FocusWish::RetainFocus = self.exec_single()? {
            let update_duration = std::time::Instant::now() - start_time;

            // If the script tries to update for more than 1ms, we move onto the next script.
            // Script updates run on the main thread, so if a script takes a long time to update,
            // the user will see it in the form of lag.
            if update_duration.as_millis() > 1 {
                break;
            }
        }

        Ok(())
    }

    /// Returns `true` if the script is ready to, and is supposed to, execute instructions.
    fn is_ready(&self) -> bool;

    /// Returns the time at which the script will be ready to run again. This is typically relevant
    /// after a `wait` instruction, which defers execution of the rest of the script until a
    /// particular amount of time has passed.
    fn wakeup_time(&self) -> GameTime;

    /// Returns the script state to an equivalent of what it would have been initialised with, so
    /// that it may be executed again in exactly the same way as it initially was.
    fn reset(&mut self);

    /// Returns this script's identity.
    fn identity(&self) -> Identity;

    /// Sets the script's state to the value given.
    fn set_state(&mut self, state: State);

    /// Returns either an owned `String` or a reference to a string containing the user-facing name
    /// of the script.
    fn name(&self) -> std::borrow::Cow<'_, str>;

    /// Adds the given flag to the script.
    fn add_flag(&mut self, flag: Flag);
}
