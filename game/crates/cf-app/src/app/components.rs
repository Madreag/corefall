use bevy::prelude::Component;

/// **M12**: marker component for slideshow UI entities. cf-app's renderer
/// owns the root node + child sprite + child text; despawning the root
/// recursively clears the whole overlay when the slideshow exits.
#[derive(Component, Debug)]
pub(crate) struct M12SlideshowRoot;

/// **M12**: marker component for the slide image entity (a child of the
/// root). cf-app updates the `ImageNode` handle when the slide index
/// changes.
#[derive(Component, Debug)]
pub(crate) struct M12SlideshowImage;

/// **M12**: marker component for the subtitle text entity. cf-app updates
/// the text string + alpha every frame from `SlideshowState`.
#[derive(Component, Debug)]
pub(crate) struct M12SlideshowSubtitle;

/// **M12**: marker component for the skip-prompt text ("Press Space to
/// skip"). Visible only while the slideshow is playing.
#[derive(Component, Debug)]
pub(crate) struct M12SlideshowSkipPrompt;

/// **M12**: marker for the slideshow music `AudioPlayer` entity. cf-app
/// despawns this entity (which Bevy interprets as "stop the sound")
/// when the slideshow exits.
#[derive(Component, Debug)]
pub(crate) struct M12SlideshowMusic;

/// **M12**: marker for the slideshow voice-over narration entity.
#[derive(Component, Debug)]
pub(crate) struct M12SlideshowVoice;

/// Marker for the screen-flash overlay entity.
#[derive(Component, Debug)]
pub(crate) struct M12ScreenFlash;
