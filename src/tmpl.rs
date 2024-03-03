use super::paths;
use hypertext::{html_elements, maud_move, Displayed, GlobalAttributes, Renderable};

fn layout<R: Renderable>(title: &'static str, content: R) -> impl Renderable {
    maud_move! {
        !DOCTYPE
        html lang="en" {
            head {
                title {
                    (Displayed(format!("{title} - Contacts.app")))
                }
                link rel="stylesheet" type="text/css" href="/assets/style.css";
            }
            body {
                h1 { "CONTACTS.APP" }
                h2 { "A Demo Contacts Application" }
                hr;
                (content)
            }
        }
    }
}

pub fn contacts(contacts: Vec<Contact>, search_term: Option<String>) -> impl Renderable {
    let content = maud_move! {
        form ."tool-bar" action=(Displayed(paths::Contacts)) method="get" {
            label for="search" { "Search Term" }
            input #search type="search" name="q" value=[search_term];
            input type="submit" value="Search";
        }
        table {
            thead {
                tr {
                    th { "First" }
                    th { "Last" }
                    th { "Phone" }
                    th { "Email" }
                    th {}
                }
            }
            tbody {
                @for contact in contacts {
                    tr {
                        td { (contact.first.as_deref().unwrap_or("")) }
                        td { (contact.last.as_deref().unwrap_or("")) }
                        td { (contact.phone.as_deref().unwrap_or("")) }
                        td { (contact.email.as_deref().unwrap_or("")) }
                        td {
                            a href=(Displayed(paths::EditContact { id: contact.id })) { "Edit" }
                            a href=(Displayed(paths::ViewContact { id: contact.id })) { "View" }
                        }
                    }
                }
            }
        }
        p {
            a href=(Displayed(paths::NewContact)) { "Add Contact" }
        }
    };
    layout("Contacts", content)
}

pub struct Contact {
    pub id: i64,
    pub first: Option<String>,
    pub last: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

pub fn new_contact() -> impl Renderable {
    let content = maud_move! {
        form action=(Displayed(paths::NewContact)) method="post" {
            fieldset {
                legend { "Contact Values" }
                p {
                    label for="first" { "First Name" }
                    input name="first" type="text" placeholder="First Name";
                    span .error {}
                }
                p {
                    label for="last" { "Last Name" }
                    input name="last" type="text" placeholder="Last Name";
                    span .error {}
                }
                p {
                    label for="phone" { "Phone" }
                    input name="phone" type="text" placeholder="Phone";
                    span .error {}
                }
                p {
                    label for="email" { "Email" }
                    input name="email" type="email" placeholder="Email";
                    span .error {}
                }
                button { "Save" }
            }
        }
        p {
            a href=(Displayed(paths::Contacts)) { "Back" }
        }
    };
    layout("New Contact", content)
}

pub fn view_contact(contact: Contact) -> impl Renderable {
    let content = maud_move! {
        h1 { (contact.first.as_deref().unwrap_or("")) " " (contact.last.as_deref().unwrap_or("")) }
        div {
            div {
                "Phone: " (contact.phone.as_deref().unwrap_or(""))
            }
            div {
                "Email: " (contact.email.as_deref().unwrap_or(""))
            }
        }
        p {
            a href=(Displayed(paths::EditContact { id: contact.id })) { "Edit" }
            a href=(Displayed(paths::Contacts)) { "Back" }
        }
    };
    layout("Contact Details", content)
}

pub fn edit_contact(contact: Contact) -> impl Renderable {
    let content = maud_move! {
        form action=(Displayed(paths::EditContact {id: contact.id })) method="post" {
            fieldset {
                legend { "Contact Values" }
                p {
                    label for="first" { "First Name" }
                    input name="first" type="text" placeholder="First Name" value=(contact.first);
                    span .error {}
                }
                p {
                    label for="last" { "Last Name" }
                    input name="last" type="text" placeholder="Last Name" value=(contact.last);
                    span .error {}
                }
                p {
                    label for="phone" { "Phone" }
                    input name="phone" type="text" placeholder="Phone" value=(contact.phone);
                    span .error {}
                }
                p {
                    label for="email" { "Email" }
                    input name="email" type="email" placeholder="Email" value=(contact.email);
                    span .error {}
                }
                button { "Save" }
            }
        }
        form action=(Displayed(paths::DeleteContact { id: contact.id })) method="post" {
            button { "Delete Contact" }
        }
        p {
            a href=(Displayed(paths::Contacts)) { "Back" }
        }

    };
    layout("Edit Contact", content)
}
