use iced::{
    widget::{button, container, image, slider, text, Column, Row, Space},
    Alignment, Color, Element, Length, Padding,
};

use crate::{
    app::{Message, Popup},
    ui::{chrome, theme::ThemeTokens},
};

#[derive(Debug, Clone, PartialEq)]
pub struct MediaPopupModel {
    pub snapshot: crate::services::media::MediaSnapshot,
    pub opacity: f32,
}

impl MediaPopupModel {
    pub fn new(snapshot: &crate::services::media::MediaSnapshot, opacity: f32) -> Self {
        Self {
            snapshot: snapshot.clone(),
            opacity,
        }
    }
}

pub fn view(theme: ThemeTokens, model: &MediaPopupModel) -> Element<'static, Message> {
    let layout = super::standard_domain_popup_layout();
    let type_scale = super::standard_popup_type_scale();
    let snap = &model.snapshot;
    let opacity = model.opacity;

    let cover = if let Some(handle) = &snap.cover_handle {
        container(
            image(handle.clone())
                .width(Length::Fill)
                .height(Length::Fixed(240.0))
                .content_fit(iced::ContentFit::Cover),
        )
    } else {
        container(
            container(text("󰝚").size(64).style(|_| iced::widget::text::Style {
                color: Some(Color::from_rgb8(0x56, 0x5f, 0x89)),
            }))
            .width(Length::Fill)
            .height(Length::Fixed(240.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(Color {
                    a: 0.1 * opacity,
                    ..Color::BLACK
                })),
                ..Default::default()
            }),
        )
    };

    let title_owned = snap.title.clone();
    let artist_owned = snap.artist.clone();
    let album_owned = snap.album.clone();

    let metadata = Column::new()
        .spacing(2)
        .push(
            text(artist_owned)
                .size(type_scale.title)
                .style(|_| iced::widget::text::Style {
                    color: Some(Color::WHITE),
                }),
        )
        .push(
            text(title_owned)
                .size(type_scale.body)
                .style(|_| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(0x7a, 0x84, 0xad)),
                }),
        )
        .push(
            text(album_owned)
                .size(type_scale.meta)
                .style(|_| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(0x56, 0x5f, 0x89)),
                }),
        );

    let play_pause_icon = if snap.playback_status == "Playing" {
        "󰏤"
    } else {
        "󰐊"
    };

    let current_secs = snap.position / 1_000_000;
    let total_secs = snap.duration / 1_000_000;

    let progress_slider = Column::new()
        .spacing(4)
        .push(
            slider(
                0.0..=(snap.duration as f64).max(1.0),
                snap.position as f64,
                |v| Message::MediaCommand(crate::services::media::MediaCommand::Seek(v as i64)),
            )
            .width(Length::Fill),
        )
        .push(
            Row::new()
                .push(
                    text(format_time(current_secs))
                        .size(type_scale.micro)
                        .style(|_| iced::widget::text::Style {
                            color: Some(Color::from_rgb8(0x56, 0x5f, 0x89)),
                        }),
                )
                .push(Space::with_width(Length::Fill))
                .push(
                    text(format_time(total_secs))
                        .size(type_scale.micro)
                        .style(|_| iced::widget::text::Style {
                            color: Some(Color::from_rgb8(0x56, 0x5f, 0x89)),
                        }),
                ),
        );

    let controls = Row::new()
        .spacing(28)
        .align_y(Alignment::Center)
        .push(
            button(text("󰒮").size(24).style(|_| iced::widget::text::Style {
                color: Some(Color::WHITE),
            }))
            .on_press(Message::MediaCommand(
                crate::services::media::MediaCommand::Previous,
            ))
            .style(move |_, _| button::Style::default()),
        )
        .push(
            button(
                text(play_pause_icon)
                    .size(42)
                    .style(|_| iced::widget::text::Style {
                        color: Some(Color::WHITE),
                    }),
            )
            .on_press(Message::MediaCommand(
                crate::services::media::MediaCommand::PlayPause,
            ))
            .style(move |_, _| button::Style::default()),
        )
        .push(
            button(text("󰒭").size(24).style(|_| iced::widget::text::Style {
                color: Some(Color::WHITE),
            }))
            .on_press(Message::MediaCommand(
                crate::services::media::MediaCommand::Next,
            ))
            .style(move |_, _| button::Style::default()),
        )
        .push(
            button(text("󰓛").size(20).style(|_| iced::widget::text::Style {
                color: Some(Color::from_rgb8(0xf7, 0x76, 0x8e)),
            }))
            .on_press(Message::MediaCommand(
                crate::services::media::MediaCommand::Stop,
            ))
            .style(move |_, _| button::Style::default()),
        );

    let volume_row = Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .push(text("󰝚").size(16).style(|_| iced::widget::text::Style {
            color: Some(Color::from_rgb8(0x56, 0x5f, 0x89)),
        }))
        .push(
            slider(0.0..=1.0, snap.volume, |v| {
                Message::MediaCommand(crate::services::media::MediaCommand::SetVolume(v))
            })
            .width(Length::Fixed(120.0)),
        );

    let content = Column::new()
        .width(Length::Fill)
        .spacing(16)
        .push(chrome::detail_popup_header_row(
            theme,
            "Media Player",
            &Popup::Media,
        ))
        .push(
            container(cover)
                .style(move |_| container::Style {
                    border: iced::Border {
                        radius: 12.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .clip(true),
        )
        .push(metadata)
        .push(progress_slider)
        .push(
            Row::new()
                .align_y(Alignment::Center)
                .push(controls)
                .push(Space::with_width(Length::Fill))
                .push(volume_row),
        );

    container(content)
        .padding(Padding::from([
            layout.outer_padding_y,
            layout.outer_padding_x,
        ]))
        .width(Length::Fixed(layout.width as f32))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(Color {
                a: opacity,
                ..Color::from_rgb8(0x1a, 0x1b, 0x26)
            })),
            border: iced::Border {
                radius: 16.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

fn format_time(seconds: i64) -> String {
    let minutes = seconds / 60;
    let seconds = seconds % 60;
    format!("{:02}:{:02}", minutes, seconds)
}
