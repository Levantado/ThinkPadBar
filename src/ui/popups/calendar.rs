use chrono::{Datelike, Local, TimeZone};
use iced::{
    widget::{button, container, text, Column, Row, Space},
    Alignment, Color, Element, Length,
};

use crate::app::Message;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarPopupModel {
    pub month_name: String,
    pub prev_icon: &'static str,
    pub next_icon: &'static str,
    pub weekday_labels: [&'static str; 7],
    pub weeks: Vec<Vec<Option<CalendarDayCell>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarDayCell {
    pub day: u32,
    pub is_today: bool,
}

impl CalendarPopupModel {
    pub fn from_offset(calendar_offset: i32, now: chrono::DateTime<Local>) -> Option<Self> {
        let mut display_month = now.month() as i32 + calendar_offset;
        let mut display_year = now.year();

        while display_month > 12 {
            display_month -= 12;
            display_year += 1;
        }
        while display_month < 1 {
            display_month += 12;
            display_year -= 1;
        }

        let display_date = Local
            .with_ymd_and_hms(display_year, display_month as u32, 1, 0, 0, 0)
            .single()?;
        let month_name = display_date.format("%B %Y").to_string();
        let current_day = if display_year == now.year() && display_month as u32 == now.month() {
            Some(now.day())
        } else {
            None
        };
        let weekday_offset = (display_date.weekday().number_from_monday() - 1) as usize;
        let days_in_month = days_in_month(display_date.year(), display_date.month());
        let weeks = calendar_weeks(weekday_offset, days_in_month, current_day);

        Some(Self {
            month_name,
            prev_icon: "<",
            next_icon: ">",
            weekday_labels: ["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Вс"],
            weeks,
        })
    }
}

pub fn view(opacity: f32, model: CalendarPopupModel) -> Element<'static, Message> {
    let title_row = Row::new()
        .spacing(10)
        .align_y(Alignment::Center)
        .push(
            button(text(model.prev_icon).size(20))
                .on_press(Message::CalendarPrevMonth)
                .style(|_, _| button::Style {
                    text_color: Color::from_rgb8(0x7a, 0xa2, 0xf7),
                    ..Default::default()
                }),
        )
        .push(
            text(model.month_name)
                .size(18)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center),
        )
        .push(
            button(text(model.next_icon).size(20))
                .on_press(Message::CalendarNextMonth)
                .style(|_, _| button::Style {
                    text_color: Color::from_rgb8(0x7a, 0xa2, 0xf7),
                    ..Default::default()
                }),
        );

    let mut header_row = Row::new().spacing(0);
    for day in model.weekday_labels {
        header_row = header_row.push(
            container(text(day).size(12).style(|_| iced::widget::text::Style {
                color: Some(Color::from_rgb8(0x56, 0x5f, 0x89)),
            }))
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center),
        );
    }

    let mut days_col = Column::new().spacing(8);
    for week in model.weeks {
        let mut week_row = Row::new().spacing(0);
        for cell in week {
            match cell {
                Some(cell) => {
                    week_row = week_row.push(day_cell(cell));
                }
                None => {
                    week_row = week_row.push(Space::with_width(Length::Fill));
                }
            }
        }
        days_col = days_col.push(week_row);
    }

    let content = Column::new()
        .spacing(16)
        .push(title_row)
        .push(header_row)
        .push(days_col);

    container(
        container(content)
            .width(Length::Fill)
            .padding(24)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(Color {
                    a: opacity,
                    ..Color::from_rgb8(0x11, 0x12, 0x1d)
                })),
                text_color: Some(Color::from_rgb8(0xc0, 0xca, 0xf5)),
                border: iced::Border {
                    radius: 12.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn day_cell(cell: CalendarDayCell) -> Element<'static, Message> {
    container(text(cell.day.to_string()).size(14))
        .width(Length::Fill)
        .padding(8)
        .align_x(iced::alignment::Horizontal::Center)
        .style(move |_| {
            if cell.is_today {
                container::Style {
                    background: Some(iced::Background::Color(Color::from_rgb8(0x7a, 0xa2, 0xf7))),
                    text_color: Some(Color::from_rgb8(0x1a, 0x1b, 0x26)),
                    border: iced::Border {
                        radius: 8.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            } else {
                container::Style::default()
            }
        })
        .into()
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn calendar_weeks(
    weekday_offset: usize,
    days_in_month: u32,
    current_day: Option<u32>,
) -> Vec<Vec<Option<CalendarDayCell>>> {
    let mut weeks = Vec::new();
    let mut current_week = vec![None; weekday_offset];

    for day in 1..=days_in_month {
        current_week.push(Some(CalendarDayCell {
            day,
            is_today: Some(day) == current_day,
        }));
        if current_week.len() == 7 {
            weeks.push(current_week);
            current_week = Vec::new();
        }
    }

    if !current_week.is_empty() {
        current_week.resize(7, None);
        weeks.push(current_week);
    }

    weeks
}

#[cfg(test)]
mod tests {
    use super::CalendarPopupModel;
    use chrono::{Local, TimeZone};

    #[test]
    fn calendar_model_builds_week_grid_and_marks_today() {
        let now = Local.with_ymd_and_hms(2026, 3, 28, 0, 32, 0).unwrap();
        let model = CalendarPopupModel::from_offset(0, now).unwrap();

        assert_eq!(model.month_name, "March 2026");
        assert_eq!(model.prev_icon, "<");
        assert_eq!(model.next_icon, ">");
        assert!(!model.weeks.is_empty());
        assert!(model
            .weeks
            .iter()
            .flat_map(|week| week.iter())
            .any(|cell| matches!(cell, Some(day) if day.day == 28 && day.is_today)));
    }
}
