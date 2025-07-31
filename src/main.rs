/*
 * Copyright 2025 mohfy, Xander
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use relm4::RelmApp;
mod config;
use gtk::{gio, glib};

mod word2ipa {
    use adw::prelude::{ActionRowExt, PreferencesGroupExt, PreferencesRowExt};
    use gtk::prelude::*;
    use relm4::prelude::*;
    use serde::Deserialize;
    use std::collections::HashMap;
    use std::error::Error;
    use std::fs::File;
    use std::io::{self, BufReader};

    //****************** edit Language here **************
    const DICT_LANG: &str = "en_US";
    //****************** edit Language here **************

    #[derive(Debug, Deserialize)]
    struct Dictionary {
        entries: Vec<HashMap<String, String>>,
    }

    pub struct Word2ipaModel {
        buffer: gtk::EntryBuffer,
        ipa_result: String,
        history: Vec<(String, String)>,
        group: adw::PreferencesGroup,
    }

    #[derive(Debug)]
    pub enum Msg {
        TextChanged,
    }

    #[relm4::component(pub)]
    impl SimpleComponent for Word2ipaModel {
        type Init = ();
        type Input = Msg;
        type Output = ();

        view! {
            #[root]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 12,
                set_margin_all: 12,
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center,

                gtk::Entry {
                    set_placeholder_text: Some("Enter a word..."),
                    set_buffer: &model.buffer,
                    connect_activate[sender] => move |_| {
                        sender.input(Msg::TextChanged);
                    },
                },

                gtk::Label {
                    #[watch]
                    set_label: &model.ipa_result,
                    set_selectable: true,
                    set_margin_all: 5,
                    add_css_class: "title-1",
                },
                adw::PreferencesPage {
                    #[name(group)]
                    adw::PreferencesGroup {
                        set_title: "History",
                    }
                }
            }
        }

        fn init(
            _init: Self::Init,
            root: Self::Root,
            sender: ComponentSender<Self>,
        ) -> ComponentParts<Self> {
            let buffer = gtk::EntryBuffer::new(None::<String>);

            let mut model = Word2ipaModel {
                ipa_result: "IPA translation will appear here.".to_string(),
                buffer,
                history: Vec::new(),
                group: adw::PreferencesGroup::new(),
            };
            let widgets = view_output!();
            model.group = widgets.group.clone();

            ComponentParts { model, widgets }
        }

        fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
            match msg {
                Msg::TextChanged => {
                    let word = self.buffer.text().to_string();
                    if word.is_empty() {
                        self.ipa_result = "IPA translation will appear here.".to_string();
                        return;
                    }
                    match word_to_ipa(&word) {
                        Ok(ipa) => {
                            self.ipa_result = ipa.clone();
                            self.history.push((word.clone(), ipa.clone()));
                            if let Some((word, ipa)) = &self.history.last() {
                                let row = adw::ActionRow::new();
                                row.set_css_classes(&["title-3"]);
                                row.set_title(ipa);
                                row.set_subtitle(word);
                                self.group.add(&row);
                            }
                        }
                        Err(err) => {
                            self.ipa_result = format!("Error: {}", err);
                            eprintln!("error: {}", err);
                        }
                    }
                }
            }
        }
    }

    fn word_to_ipa(word: &str) -> Result<String, Box<dyn Error>> {
        let resource_data = gtk::gio::resources_lookup_data(
            &format!("/com/mohfy/word2ipa/dicts/{DICT_LANG}.json"),
            gtk::gio::ResourceLookupFlags::NONE,
        )
        .map_err(|e| format!("Failed to load resource: {}", e))?;

        let json_str = std::str::from_utf8(&resource_data)
            .map_err(|e| format!("Invalid UTF-8 in resource: {}", e))?;

        let dictionary: Dictionary =
            serde_json::from_str(json_str).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        if let Some(first_map) = dictionary.entries.get(0) {
            if let Some(ipa) = first_map.get(&word.to_lowercase()) {
                Ok(ipa.clone())
            } else {
                Err(format!("Word '{}' not found.", word).into())
            }
        } else {
            Err("Dictionary format error.".into())
        }
    }
}

mod ipa_dictionary {
    use gtk::prelude::*;
    use relm4::adw;
    use relm4::prelude::*;
    use serde::Deserialize;

    // Required for methods like set_title, set_subtitle, add
    use adw::prelude::{ActionRowExt, PreferencesGroupExt, PreferencesRowExt};

    #[derive(Debug, Clone, Deserialize)]
    pub struct IpaEntry {
        pub symbol: String,
        pub sound: String,
        pub description: String,
        pub examples: Vec<String>,
        pub ipa_examples: Vec<String>,
    }

    pub struct IpaDictionaryModel {
        entries: Vec<IpaEntry>,
    }

    #[derive(Debug)]
    pub enum Msg {}

    #[relm4::component(pub)]
    impl SimpleComponent for IpaDictionaryModel {
        type Init = ();
        type Input = Msg;
        type Output = ();

        view! {
            #[root]
            adw::PreferencesPage {
                #[name(group)]
                adw::PreferencesGroup {
                    set_title: "IPA Symbols",
                }
            }
        }

        fn init(
            _init: Self::Init,
            root: Self::Root,
            _sender: ComponentSender<Self>,
        ) -> ComponentParts<Self> {
            let entries = match load_ipa_entries() {
                Ok(entries) => entries,
                Err(err) => {
                    eprintln!("Error loading IPA dictionary: {}", err);
                    Vec::new()
                }
            };

            let model = IpaDictionaryModel { entries };
            let widgets = view_output!();
            let group = &widgets.group;

            for entry in &model.entries {
                let row = adw::ActionRow::new();
                row.set_css_classes(&["title-4"]);
                row.set_title(&format!("{} – {}", entry.symbol, entry.sound));
                row.set_subtitle(&entry.description);

                let examples_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
                for (word, ipa) in entry.examples.iter().zip(entry.ipa_examples.iter()) {
                    let label = gtk::Label::new(Some(&format!("• {} {}", word, ipa)));
                    label.set_xalign(0.0);
                    label.set_margin_top(2);
                    label.set_margin_bottom(2);
                    examples_box.append(&label);
                }

                row.add_suffix(&examples_box); // visually aligns better than prefix
                group.add(&row);
            }

            ComponentParts { model, widgets }
        }

        fn update(&mut self, _msg: Self::Input, _sender: ComponentSender<Self>) {}
    }

    fn load_ipa_entries() -> Result<Vec<IpaEntry>, Box<dyn std::error::Error>> {
        let resource_data = gtk::gio::resources_lookup_data(
            "/com/mohfy/word2ipa/dicts/ipa_lookup_table.json",
            gtk::gio::ResourceLookupFlags::NONE,
        )
        .map_err(|e| format!("Failed to load IPA resource: {}", e))?;

        let json_str = std::str::from_utf8(&resource_data)
            .map_err(|e| format!("Invalid UTF-8 in IPA resource: {}", e))?;

        let entries = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse IPA JSON: {}", e))?;

        Ok(entries)
    }
}

mod app {
    use super::ipa_dictionary::IpaDictionaryModel;
    use super::word2ipa::Word2ipaModel;
    use relm4::adw::prelude::*;
    use relm4::prelude::*;

    pub struct App {
        _word2ipa: Controller<Word2ipaModel>,
        _ipa_dict: Controller<IpaDictionaryModel>,
    }

    #[derive(Debug)]
    pub enum Msg {}

    #[relm4::component(pub)]
    impl SimpleComponent for App {
        type Init = ();
        type Input = Msg;
        type Output = ();

        view! {
            #[root]
            adw::ApplicationWindow {
                set_title: Some("IPA Dictionary"),
                set_default_size: (600, 700),
                set_content: Some(&toolbar_view),
            },

            toolbar_view = adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    #[wrap(Some)]
                    set_title_widget = &adw::ViewSwitcher {
                        set_stack: Some(&stack),
                    }
                },
                set_content: Some(&stack),
            },

            stack = &adw::ViewStack {
                add_titled_with_icon: (word2ipa.widget(), Some("word2ipa"), "Word to IPA", "edit-find-symbolic"),
                add_titled_with_icon: (ipa_dict.widget(), Some("dictionary"), "IPA Dictionary", "view-list-symbolic"),
            }
        }

        fn init(
            _init: Self::Init,
            root: Self::Root,
            _sender: ComponentSender<Self>,
        ) -> ComponentParts<Self> {
            let word2ipa = Word2ipaModel::builder().launch(()).detach();
            let ipa_dict = IpaDictionaryModel::builder().launch(()).detach();
            let widgets = view_output!();

            let model = App {
                _word2ipa: word2ipa,
                _ipa_dict: ipa_dict,
            };

            ComponentParts { model, widgets }
        }

        fn update(&mut self, _msg: Self::Input, _sender: ComponentSender<Self>) {}
    }
}

fn main() {
    let res = gio::Resource::load(config::PKGDATADIR.to_owned() + "/word2ipa.gresource")
        .expect("Failed to initialize the resource file.");
    gio::resources_register(&res);

    let app = RelmApp::new("io.github.mohfy.word2ipa");
    app.run::<app::App>(());
}
