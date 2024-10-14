use crate::{
    apps::DesktopList,
    common::{DesktopEntry, DesktopHandler, Handleable},
    error::{Error, Result},
};
use mime::Mime;
use std::{collections::BTreeMap, convert::TryFrom, ffi::OsString};

#[derive(Debug, Default, Clone)]
pub struct SystemApps {
    /// Associations of mimes and lists of apps
    pub associations: BTreeMap<Mime, DesktopList>,
    /// Apps with no associated mime
    unassociated: DesktopList,
}

impl SystemApps {
    /// Get the list of handlers associated with a given mime
    pub fn get_handlers(&self, mime: &Mime) -> Result<DesktopList> {
        Ok(self
            .associations
            .get(mime)
            .ok_or_else(|| Error::NotFound(mime.to_string()))?
            .clone())
    }

    /// Get the primary of handler associated with a given mime
    pub fn get_handler(&self, mime: &Mime) -> Result<DesktopHandler> {
        Ok(self
            .get_handlers(mime)?
            .front()
            .ok_or_else(|| Error::NotFound(mime.to_string()))?
            .clone())
    }

    /// Get all system-level desktop entries on the system
    #[mutants::skip] // Cannot test directly, depends on system state
    pub fn get_entries(
    ) -> Result<impl Iterator<Item = (OsString, DesktopEntry)>> {
        Ok(xdg::BaseDirectories::new()?
            .list_data_files_once("applications")
            .into_iter()
            .filter(|p| {
                p.extension().and_then(|x| x.to_str()) == Some("desktop")
            })
            .filter_map(|p| {
                Some((
                    p.file_name()?.to_owned(),
                    DesktopEntry::try_from(p.clone()).ok()?,
                ))
            }))
    }

    /// Create a new instance of `SystemApps`
    #[mutants::skip] // Cannot test directly, depends on system state
    pub fn populate() -> Result<Self> {
        let mut associations = BTreeMap::<Mime, DesktopList>::new();
        let mut unassociated = DesktopList::default();

        Self::get_entries()?.for_each(|(_, entry)| {
            let (file_name, mimes) = (entry.file_name, entry.mime_type);
            let desktop_handler =
                DesktopHandler::assume_valid(file_name.to_owned());

            if mimes.is_empty() {
                unassociated.push_back(desktop_handler);
            } else {
                mimes.into_iter().for_each(|mime| {
                    associations
                        .entry(mime)
                        .or_default()
                        .push_back(desktop_handler.clone());
                });
            }
        });

        Ok(Self {
            associations,
            unassociated,
        })
    }

    /// Get an installed terminal emulator
    pub fn terminal_emulator(&self) -> Option<DesktopEntry> {
        self.unassociated
            .iter()
            .filter_map(|h| h.get_entry().ok())
            .find(|h| h.is_terminal_emulator())
    }

    #[cfg(test)]
    /// Internal helper function for testing
    pub fn add_unassociated(&mut self, handler: DesktopHandler) {
        self.unassociated.push_front(handler)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_handlers() -> Result<()> {
        let mut expected_handlers = DesktopList::default();
        expected_handlers
            .push_back(DesktopHandler::assume_valid("helix.desktop".into()));
        expected_handlers
            .push_back(DesktopHandler::assume_valid("nvim.desktop".into()));

        let mut associations: BTreeMap<Mime, DesktopList> = BTreeMap::new();

        associations.insert(mime::TEXT_PLAIN, expected_handlers.clone());

        let system_apps = SystemApps {
            associations,
            ..Default::default()
        };

        assert_eq!(
            system_apps
                .get_handler(&mime::TEXT_PLAIN)
                .expect("Could not get handler")
                .to_string(),
            "helix.desktop"
        );
        assert_eq!(
            system_apps
                .get_handlers(&mime::TEXT_PLAIN)
                .expect("Could not get handler"),
            expected_handlers
        );

        Ok(())
    }
}
