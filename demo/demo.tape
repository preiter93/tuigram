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

# Add participants: Client(0), Gateway(1), Auth(2), Orders(3)
Type "p"
Sleep 600ms
Type "Client"
Sleep 300ms
Enter
Sleep 600ms

Type "p"
Sleep 600ms
Type "Gateway"
Sleep 300ms
Enter
Sleep 600ms

Type "p"
Sleep 600ms
Type "Auth"
Sleep 300ms
Enter
Sleep 600ms

Type "p"
Sleep 600ms
Type "Orders"
Sleep 300ms
Enter
Sleep 1s

# Client(0) -> Gateway(1): Request
Type "e"
Sleep 600ms
Enter
Sleep 400ms
Enter
Sleep 400ms
Type "Request"
Sleep 300ms
Enter
Sleep 800ms

# Gateway(1) -> Auth(2): Validate Token
Type "e"
Sleep 600ms
Down
Sleep 300ms
Enter
Sleep 400ms
Down Down
Sleep 300ms
Enter
Sleep 400ms
Type "Validate Token"
Sleep 300ms
Enter
Sleep 800ms

# Auth(2) -> Gateway(1): Token Valid
Type "e"
Sleep 600ms
Down Down
Sleep 300ms
Enter
Sleep 400ms
Down
Sleep 300ms
Enter
Sleep 400ms
Type "Token Valid"
Sleep 300ms
Enter
Sleep 800ms

# Gateway(1) -> Orders(3): Get Orders
Type "e"
Sleep 600ms
Down
Sleep 300ms
Enter
Sleep 400ms
Down Down Down
Sleep 300ms
Enter
Sleep 400ms
Type "Get Orders"
Sleep 300ms
Enter
Sleep 800ms

# Orders(3) -> Gateway(1): Order List
Type "e"
Sleep 600ms
Down Down Down
Sleep 300ms
Enter
Sleep 400ms
Down
Sleep 300ms
Enter
Sleep 400ms
Type "Order List"
Sleep 300ms
Enter
Sleep 800ms

# Gateway(1) -> Client(0): Response
Type "e"
Sleep 600ms
Down
Sleep 300ms
Enter
Sleep 400ms
Enter
Sleep 400ms
Type "Response"
Sleep 300ms
Enter
Sleep 1.5s

# Navigate through events with j/k
Type "j"
Sleep 400ms
Type "j"
Sleep 400ms
Type "j"
Sleep 400ms
Type "k"
Sleep 400ms

# Navigate through participants with h/l
Type "l"
Sleep 400ms
Type "l"
Sleep 400ms
Type "l"
Sleep 400ms
Type "h"
Sleep 400ms

# Move a participant with H/L
Type "L"
Sleep 500ms
Type "H"
Sleep 500ms

# Select an event and move it with J/K
Type "j"
Sleep 400ms
Type "J"
Sleep 500ms
Type "K"
Sleep 500ms

# Show help
Type "?"
Sleep 2s
Type "?"
Sleep 600ms

# Export to Mermaid
Type "m"
Sleep 1.2s

# Quit
Ctrl+c
Sleep 600ms

Type "cat diagram.mmd" Enter
Sleep 3s
