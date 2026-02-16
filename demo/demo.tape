# Tuigram Demo Recording
#
# Generate the demo GIF:
#   vhs demo.tape
#
# VHS:
#   https://github.com/charmbracelet/vhs

Output demo.gif

Require tuigram

Set Shell "bash"
Set FontSize 16
Set Width 1000
Set Height 600
Set Padding 20
Set TypingSpeed 80ms
Set Theme "Dracula"

Hide
Type "tuigram" Enter
Sleep 500ms
Show

Sleep 1.5s

# Add first participant
Type "p"
Sleep 800ms
Type "User"
Sleep 300ms
Enter
Sleep 800ms

# Add second participant
Type "p"
Sleep 800ms
Type "API"
Sleep 300ms
Enter
Sleep 800ms

# Add third participant
Type "p"
Sleep 800ms
Type "Database"
Sleep 300ms
Enter
Sleep 1s

# Add first event
Type "e"
Sleep 800ms
Enter
Sleep 500ms
Down
Sleep 300ms
Enter
Sleep 500ms
Type "Login Request"
Sleep 300ms
Enter
Sleep 1s

# Add second event
Type "e"
Sleep 800ms
Down
Sleep 300ms
Enter
Sleep 500ms
Down Down
Sleep 300ms
Enter
Sleep 500ms
Type "Query User"
Sleep 300ms
Enter
Sleep 1s

# Add third event
Type "e"
Sleep 800ms
Down Down
Sleep 300ms
Enter
Sleep 500ms
Down
Sleep 300ms
Enter
Sleep 500ms
Type "User Data"
Sleep 300ms
Enter
Sleep 1s

# Add fourth event
Type "e"
Sleep 800ms
Down
Sleep 300ms
Enter
Sleep 500ms
Enter
Sleep 500ms
Type "Login Success"
Sleep 300ms
Enter
Sleep 1.5s

# Navigate through events
Type "j"
Sleep 500ms
Type "j"
Sleep 500ms
Type "j"
Sleep 500ms
Type "k"
Sleep 500ms

# Navigate through participants
Type "l"
Sleep 500ms
Type "l"
Sleep 500ms
Type "h"
Sleep 500ms

# Move a participant
Type "L"
Sleep 600ms
Type "H"
Sleep 600ms

# Select an event and move it
Type "j"
Sleep 500ms
Type "J"
Sleep 600ms
Type "K"
Sleep 600ms

# Show help
Type "?"
Sleep 2s
Type "?"
Sleep 800ms

# Export to Mermaid
Type "m"
Sleep 1.5s

# Quit
Ctrl+c
Sleep 800ms

Type "cat diagram.mmd" Enter
Sleep 3s
