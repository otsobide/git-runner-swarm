use std::fmt;
use std::sync::Arc;

use swarm_domain::Forge;

use crate::ports::RunnerImage;

/// The runner images runners can be created from, each under its own key.
///
/// The first image given for a forge is that forge's default, the one a
/// runner is created from when no image is asked for.
#[derive(Clone, Default)]
pub struct RunnerImages(Vec<Arc<dyn RunnerImage>>);

impl RunnerImages {
    /// # Panics
    ///
    /// When two images share a key: the panel could not tell them apart.
    pub fn new(images: impl IntoIterator<Item = Arc<dyn RunnerImage>>) -> Self {
        let images: Vec<Arc<dyn RunnerImage>> = images.into_iter().collect();
        for (index, image) in images.iter().enumerate() {
            assert!(
                images[..index]
                    .iter()
                    .all(|earlier| earlier.key() != image.key()),
                "two runner images share the key `{}`",
                image.key()
            );
        }
        Self(images)
    }

    /// The image with this key.
    pub fn find(&self, key: &str) -> Option<&dyn RunnerImage> {
        self.iter().find(|image| image.key() == key)
    }

    /// The image runners of `forge` are created from unless another is
    /// asked for: the first one given for it.
    pub fn default_for(&self, forge: Forge) -> Option<&dyn RunnerImage> {
        self.iter().find(|image| image.forge() == forge)
    }

    /// Every image, in the order given.
    pub fn iter(&self) -> impl Iterator<Item = &dyn RunnerImage> {
        self.0.iter().map(AsRef::as_ref)
    }
}

impl fmt::Debug for RunnerImages {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.iter().map(RunnerImage::key))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeImage;

    fn images() -> RunnerImages {
        RunnerImages::new([
            Arc::new(FakeImage::new("github-official", Forge::GitHub)) as Arc<dyn RunnerImage>,
            Arc::new(FakeImage::new("github-custom", Forge::GitHub)),
        ])
    }

    #[test]
    fn finds_an_image_by_its_key() {
        let images = images();

        assert_eq!(
            images.find("github-custom").map(RunnerImage::key),
            Some("github-custom")
        );
        assert!(images.find("gitea").is_none());
    }

    #[test]
    fn defaults_to_the_first_image_given_for_a_forge() {
        let images = images();

        assert_eq!(
            images.default_for(Forge::GitHub).map(RunnerImage::key),
            Some("github-official")
        );
    }

    #[test]
    fn has_no_default_for_a_forge_it_has_no_image_for() {
        assert!(RunnerImages::default().default_for(Forge::GitHub).is_none());
    }

    #[test]
    #[should_panic(expected = "two runner images share the key `github-official`")]
    fn refuses_two_images_under_one_key() {
        let _ = RunnerImages::new([
            Arc::new(FakeImage::new("github-official", Forge::GitHub)) as Arc<dyn RunnerImage>,
            Arc::new(FakeImage::new("github-official", Forge::GitHub)),
        ]);
    }
}
