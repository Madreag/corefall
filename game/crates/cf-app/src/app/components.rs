use bevy::prelude::Component;

/// owns the root node + child sprite + child text; despawning the root
/// recursively clears the whole overlay when the slideshow exits.
#[derive(Component, Debug)]
pub(crate) struct M12SlideshowRoot;

/// root). cf-app updates the `ImageNode` handle when the slide index
/// changes.
#[derive(Component, Debug)]
pub(crate) struct M12SlideshowImage;

/// the text string + alpha every frame from `SlideshowState`.
#[derive(Component, Debug)]
pub(crate) struct M12SlideshowSubtitle;

/// skip"). Visible only while the slideshow is playing.
#[derive(Component, Debug)]
pub(crate) struct M12SlideshowSkipPrompt;

/// despawns this entity (which Bevy interprets as "stop the sound")
/// when the slideshow exits.
#[derive(Component, Debug)]
pub(crate) struct M12SlideshowMusic;

#[derive(Component, Debug)]
pub(crate) struct M12SlideshowVoice;

/// Marker for the screen-flash overlay entity.
#[derive(Component, Debug)]
pub(crate) struct M12ScreenFlash;
