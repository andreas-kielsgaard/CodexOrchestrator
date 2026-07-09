# Agent Session Views

Agent Session views render the session launcher, runtime choices, prompt form,
attachments, and conversation output.

Views may own local form input when it is purely presentational, but workflow
coordination and runtime calls should move into a feature controller as this
feature is split further.
