use iced::widget::{button, checkbox, column, container, radio, row, space, text};
use iced::{Element, Fill, Size, Task, Theme};
use msi_gpu_mode::{Machine, Mode, ModeVar, system};
use std::env;
use std::path::PathBuf;
use std::process::Command;

const PKEXEC_DISMISSED: i32 = 126;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("MSI GPU Mode")
        .window_size(Size::new(440.0, 430.0))
        .resizable(false)
        .run()
}

struct App {
    machine: Machine,
    active: Option<Mode>,
    stored: Result<ModeVar, String>,
    concerns: Vec<&'static str>,
    selected: Option<Mode>,
    acknowledged: bool,
    busy: bool,
    error: Option<String>,
}

#[derive(Debug, Clone)]
enum Message {
    Select(Mode),
    Acknowledge(bool),
    Apply,
    Applied(Result<(), String>),
    Reboot,
}

impl App {
    fn new() -> Self {
        let machine = Machine::detect();
        let stored = if machine.is_msi() {
            ModeVar::load().map_err(|e| e.to_string())
        } else {
            Err(format!("Not an MSI machine ({}).", machine.vendor))
        };
        let concerns = stored
            .as_ref()
            .map(|var| system::concerns(&machine, var))
            .unwrap_or_default();
        Self {
            active: system::active_mode(&system::gpus()),
            selected: stored.as_ref().ok().map(ModeVar::mode),
            machine,
            stored,
            concerns,
            acknowledged: false,
            busy: false,
            error: None,
        }
    }

    fn stored_mode(&self) -> Option<Mode> {
        self.stored.as_ref().ok().map(ModeVar::mode)
    }

    fn can_apply(&self) -> bool {
        !self.busy
            && self.selected.is_some()
            && self.selected != self.stored_mode()
            && (self.concerns.is_empty() || self.acknowledged)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Select(mode) => {
                self.selected = Some(mode);
                self.error = None;
            }
            Message::Acknowledge(value) => self.acknowledged = value,
            Message::Apply => {
                let Some(mode) = self.selected else {
                    return Task::none();
                };
                self.busy = true;
                self.error = None;
                let force = !self.concerns.is_empty();
                return Task::perform(async move { apply(mode, force) }, Message::Applied);
            }
            Message::Applied(result) => {
                self.busy = false;
                self.error = result.err();
                self.stored = ModeVar::load().map_err(|e| e.to_string());
                if self.error.is_some() {
                    self.selected = self.stored_mode();
                }
            }
            Message::Reboot => {
                if let Err(e) = Command::new("systemctl").arg("reboot").spawn() {
                    self.error = Some(format!("Could not reboot: {e}"));
                }
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let header = column![
            text(&self.machine.product).size(20),
            text(format!("{}  /  BIOS {}", self.machine.board, self.machine.bios))
                .size(13)
                .style(text::secondary),
        ]
        .spacing(4);

        let body: Element<_> = match &self.stored {
            Ok(_) => self.modes(),
            Err(reason) => text(reason).size(14).style(text::secondary).into(),
        };

        container(column![header, body].spacing(20))
            .padding(24)
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn modes(&self) -> Element<'_, Message> {
        let mut content = column![].spacing(8);
        for mode in Mode::ALL {
            content = content.push(self.mode_card(mode));
        }

        if !self.concerns.is_empty() {
            content = content.push(space().height(4)).push(
                checkbox(self.acknowledged)
                    .label(format!("Untested here: {}. Switch anyway.", self.concerns.join(", ")))
                    .on_toggle(Message::Acknowledge)
                    .size(16)
                    .text_size(13),
            );
        }

        let (note, note_style) = self.note();
        let pending = self.stored_mode().is_some() && self.stored_mode() != self.active;

        let mut actions = row![space::horizontal()].spacing(8);
        if pending && !self.busy {
            actions = actions.push(
                button(text("Reboot").size(14))
                    .padding([8, 18])
                    .style(button::secondary)
                    .on_press(Message::Reboot),
            );
        }
        actions = actions.push(
            button(text(if self.busy { "Applying" } else { "Apply" }).size(14))
                .padding([8, 18])
                .on_press_maybe(self.can_apply().then_some(Message::Apply)),
        );

        content
            .push(space::vertical())
            .push(text(note).size(13).style(note_style))
            .push(actions)
            .into()
    }

    fn mode_card(&self, mode: Mode) -> Element<'_, Message> {
        let label = if self.active == Some(mode) {
            format!("{} (in use)", mode.title())
        } else {
            mode.title().to_string()
        };
        let choice = radio(label, mode, self.selected, Message::Select).size(16).text_size(15);

        container(
            column![
                choice,
                row![space().width(24), text(mode.summary()).size(13).style(text::secondary)],
            ]
            .spacing(4),
        )
        .width(Fill)
        .padding([12, 14])
        .style(container::rounded_box)
        .into()
    }

    fn note(&self) -> (String, fn(&Theme) -> text::Style) {
        if let Some(error) = &self.error {
            return (error.clone(), text::danger);
        }
        match (self.stored_mode(), self.active) {
            (Some(stored), active) if Some(stored) != active => (
                format!("{} is set for the next boot. Reboot to switch.", stored.title()),
                text::warning,
            ),
            _ => ("Changes take effect after a reboot.".to_string(), text::secondary),
        }
    }
}

// The GUI stays unprivileged. Only the CLI runs as root, through a polkit prompt.
fn apply(mode: Mode, force: bool) -> Result<(), String> {
    let mut command = Command::new("pkexec");
    command.arg(cli_path()).arg("set").arg(mode.id());
    if force {
        command.arg("--force");
    }
    let output = command.output().map_err(|e| format!("Could not run pkexec: {e}"))?;
    if output.status.success() || output.status.code() == Some(PKEXEC_DISMISSED) {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let message = stderr.lines().last().unwrap_or("the helper failed").trim();
    Err(message.trim_start_matches("error: ").to_string())
}

fn cli_path() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|exe| Some(exe.parent()?.join("msi-gpu-mode")))
        .filter(|path| path.exists())
        .unwrap_or_else(|| PathBuf::from("msi-gpu-mode"))
}
