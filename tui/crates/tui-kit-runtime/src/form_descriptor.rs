//! Shared form descriptors for Create/Insert.
//! Centralizes field semantics so host, runtime, and render can share one definition.

use crate::{
    CoreAction, CoreState, CreateModalFocus, InsertFormFocus, InsertMode,
    kinic_tabs::{KINIC_CREATE_TAB_ID, KINIC_INSERT_TAB_ID},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormKind {
    Create,
    Insert,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormResetKind {
    Create,
    Insert,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormFocus {
    Create(CreateModalFocus),
    Insert(InsertFormFocus),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormCommand {
    Input(char),
    Backspace,
    NextField,
    PrevField,
    Submit,
    HorizontalChangePrev,
    HorizontalChangeNext,
    OpenPicker,
    OpenFileDialog,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormDescriptor {
    pub kind: FormKind,
    pub first_focus: FormFocus,
    pub reset_kind: FormResetKind,
}

impl FormDescriptor {
    pub fn next_command(self) -> FormCommand {
        FormCommand::NextField
    }

    pub fn prev_command(self) -> FormCommand {
        FormCommand::PrevField
    }

    pub fn backspace_command(self) -> FormCommand {
        FormCommand::Backspace
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FormFieldSpec {
    focus: FormFocus,
    enter_command: FormCommand,
    accepts_input: bool,
    supports_horizontal_change: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InsertFieldAvailability {
    Always,
    FileOnly,
    TextModes,
    ManualEmbeddingOnly,
}
impl InsertFieldAvailability {
    fn matches(self, mode: InsertMode) -> bool {
        match self {
            Self::Always => true,
            Self::FileOnly => mode == InsertMode::File,
            Self::TextModes => matches!(mode, InsertMode::InlineText | InsertMode::ManualEmbedding),
            Self::ManualEmbeddingOnly => mode == InsertMode::ManualEmbedding,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InsertFieldSpec {
    field: FormFieldSpec,
    availability: InsertFieldAvailability,
}
const CREATE_DESCRIPTOR: FormDescriptor = FormDescriptor {
    kind: FormKind::Create,
    first_focus: FormFocus::Create(CreateModalFocus::Name),
    reset_kind: FormResetKind::Create,
};
const INSERT_DESCRIPTOR: FormDescriptor = FormDescriptor {
    kind: FormKind::Insert,
    first_focus: FormFocus::Insert(InsertFormFocus::Mode),
    reset_kind: FormResetKind::Insert,
};
const CREATE_FIELDS: [FormFieldSpec; 3] = [
    FormFieldSpec {
        focus: FormFocus::Create(CreateModalFocus::Name),
        enter_command: FormCommand::NextField,
        accepts_input: true,
        supports_horizontal_change: false,
    },
    FormFieldSpec {
        focus: FormFocus::Create(CreateModalFocus::Description),
        enter_command: FormCommand::NextField,
        accepts_input: true,
        supports_horizontal_change: false,
    },
    FormFieldSpec {
        focus: FormFocus::Create(CreateModalFocus::Submit),
        enter_command: FormCommand::Submit,
        accepts_input: false,
        supports_horizontal_change: false,
    },
];
const INSERT_FIELDS: [InsertFieldSpec; 7] = [
    InsertFieldSpec {
        field: FormFieldSpec {
            focus: FormFocus::Insert(InsertFormFocus::Mode),
            enter_command: FormCommand::NextField,
            accepts_input: false,
            supports_horizontal_change: true,
        },
        availability: InsertFieldAvailability::Always,
    },
    InsertFieldSpec {
        field: FormFieldSpec {
            focus: FormFocus::Insert(InsertFormFocus::MemoryId),
            enter_command: FormCommand::OpenPicker,
            accepts_input: false,
            supports_horizontal_change: false,
        },
        availability: InsertFieldAvailability::Always,
    },
    InsertFieldSpec {
        field: FormFieldSpec {
            focus: FormFocus::Insert(InsertFormFocus::Tag),
            enter_command: FormCommand::NextField,
            accepts_input: true,
            supports_horizontal_change: false,
        },
        availability: InsertFieldAvailability::Always,
    },
    InsertFieldSpec {
        field: FormFieldSpec {
            focus: FormFocus::Insert(InsertFormFocus::Text),
            enter_command: FormCommand::NextField,
            accepts_input: true,
            supports_horizontal_change: false,
        },
        availability: InsertFieldAvailability::TextModes,
    },
    InsertFieldSpec {
        field: FormFieldSpec {
            focus: FormFocus::Insert(InsertFormFocus::FilePath),
            enter_command: FormCommand::OpenFileDialog,
            accepts_input: true,
            supports_horizontal_change: false,
        },
        availability: InsertFieldAvailability::FileOnly,
    },
    InsertFieldSpec {
        field: FormFieldSpec {
            focus: FormFocus::Insert(InsertFormFocus::Embedding),
            enter_command: FormCommand::NextField,
            accepts_input: true,
            supports_horizontal_change: false,
        },
        availability: InsertFieldAvailability::ManualEmbeddingOnly,
    },
    InsertFieldSpec {
        field: FormFieldSpec {
            focus: FormFocus::Insert(InsertFormFocus::Submit),
            enter_command: FormCommand::Submit,
            accepts_input: false,
            supports_horizontal_change: false,
        },
        availability: InsertFieldAvailability::Always,
    },
];
pub fn form_descriptor(tab_id: &str) -> Option<FormDescriptor> {
    match tab_id {
        KINIC_CREATE_TAB_ID => Some(CREATE_DESCRIPTOR),
        KINIC_INSERT_TAB_ID => Some(INSERT_DESCRIPTOR),
        _ => None,
    }
}
pub fn apply_form_focus(state: &mut CoreState, focus: FormFocus) {
    match focus {
        FormFocus::Create(focus) => state.create_focus = focus,
        FormFocus::Insert(focus) => state.insert_focus = focus,
    }
}
pub fn current_form_focus(state: &CoreState) -> Option<FormFocus> {
    current_form_focus_for_tab(
        state.current_tab_id.as_str(),
        state.create_focus,
        state.insert_focus,
    )
}
pub fn current_form_focus_for_tab(
    tab_id: &str,
    create_focus: CreateModalFocus,
    insert_focus: InsertFormFocus,
) -> Option<FormFocus> {
    match form_descriptor(tab_id)?.kind {
        FormKind::Create => Some(FormFocus::Create(create_focus)),
        FormKind::Insert => Some(FormFocus::Insert(insert_focus)),
    }
}
pub fn form_enter_command(state: &CoreState) -> Option<FormCommand> {
    current_form_field_spec(state).map(|field| field.enter_command)
}
pub fn form_char_input_command(state: &CoreState, c: char) -> Option<FormCommand> {
    current_form_field_spec(state)
        .filter(|field| field.accepts_input)
        .map(|_| FormCommand::Input(c))
}
pub fn form_horizontal_change_command(focus: FormFocus, forward: bool) -> Option<FormCommand> {
    form_field_spec(focus, None)
        .filter(|field| field.supports_horizontal_change)
        .map(|_| {
            if forward {
                FormCommand::HorizontalChangeNext
            } else {
                FormCommand::HorizontalChangePrev
            }
        })
}
pub fn form_command_to_action(kind: FormKind, command: FormCommand) -> Option<CoreAction> {
    match (kind, command) {
        (FormKind::Create, FormCommand::Input(c)) => Some(CoreAction::CreateInput(c)),
        (FormKind::Create, FormCommand::Backspace) => Some(CoreAction::CreateBackspace),
        (FormKind::Create, FormCommand::NextField) => Some(CoreAction::CreateNextField),
        (FormKind::Create, FormCommand::PrevField) => Some(CoreAction::CreatePrevField),
        (FormKind::Create, FormCommand::Submit) => Some(CoreAction::CreateSubmit),
        (FormKind::Insert, FormCommand::Input(c)) => Some(CoreAction::InsertInput(c)),
        (FormKind::Insert, FormCommand::Backspace) => Some(CoreAction::InsertBackspace),
        (FormKind::Insert, FormCommand::NextField) => Some(CoreAction::InsertNextField),
        (FormKind::Insert, FormCommand::PrevField) => Some(CoreAction::InsertPrevField),
        (FormKind::Insert, FormCommand::Submit) => Some(CoreAction::InsertSubmit),
        (FormKind::Insert, FormCommand::HorizontalChangePrev) => Some(CoreAction::InsertPrevMode),
        (FormKind::Insert, FormCommand::HorizontalChangeNext) => Some(CoreAction::InsertNextMode),
        (FormKind::Insert, FormCommand::OpenPicker) => Some(CoreAction::OpenDefaultMemoryPicker),
        (FormKind::Insert, FormCommand::OpenFileDialog) => Some(CoreAction::InsertOpenFileDialog),
        _ => None,
    }
}
pub fn core_action_to_form_command(action: &CoreAction) -> Option<(FormKind, FormCommand)> {
    match action {
        CoreAction::InsertInput(c) => Some((FormKind::Insert, FormCommand::Input(*c))),
        CoreAction::InsertBackspace => Some((FormKind::Insert, FormCommand::Backspace)),
        CoreAction::InsertNextField => Some((FormKind::Insert, FormCommand::NextField)),
        CoreAction::InsertPrevField => Some((FormKind::Insert, FormCommand::PrevField)),
        CoreAction::InsertSubmit => Some((FormKind::Insert, FormCommand::Submit)),
        CoreAction::InsertPrevMode => Some((FormKind::Insert, FormCommand::HorizontalChangePrev)),
        CoreAction::InsertNextMode => Some((FormKind::Insert, FormCommand::HorizontalChangeNext)),
        CoreAction::OpenDefaultMemoryPicker => Some((FormKind::Insert, FormCommand::OpenPicker)),
        CoreAction::InsertOpenFileDialog => Some((FormKind::Insert, FormCommand::OpenFileDialog)),
        CoreAction::CreateInput(c) => Some((FormKind::Create, FormCommand::Input(*c))),
        CoreAction::CreateBackspace => Some((FormKind::Create, FormCommand::Backspace)),
        CoreAction::CreateNextField => Some((FormKind::Create, FormCommand::NextField)),
        CoreAction::CreatePrevField => Some((FormKind::Create, FormCommand::PrevField)),
        CoreAction::CreateSubmit => Some((FormKind::Create, FormCommand::Submit)),
        _ => None,
    }
}
pub fn form_shows_horizontal_change_hint(is_form_focus: bool, focus: FormFocus) -> bool {
    is_form_focus
        && form_field_spec(focus, None).is_some_and(|field| field.supports_horizontal_change)
}
fn current_form_field_spec(state: &CoreState) -> Option<FormFieldSpec> {
    let focus = current_form_focus(state)?;
    form_field_spec(focus, Some(state.insert_mode))
}
fn form_field_spec(focus: FormFocus, insert_mode: Option<InsertMode>) -> Option<FormFieldSpec> {
    match focus {
        FormFocus::Create(_) => CREATE_FIELDS.iter().find(|field| field.focus == focus).copied(),
        FormFocus::Insert(_) => {
            let mode = insert_mode.unwrap_or_default();
            INSERT_FIELDS
                .iter()
                .find(|field| field.field.focus == focus && field.availability.matches(mode))
                .map(|field| field.field)
        }
    }
}
