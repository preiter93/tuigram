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

# Add participants
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

# Client -> Gateway: Request
Type "e"
Sleep 600ms
Enter
Sleep 400ms
Down
Sleep 300ms
Enter
Sleep 400ms
Type "Request"
Sleep 300ms
Enter
Sleep 800ms

# Gateway -> Auth: Validate Token
Type "e"
Sleep 600ms
Down
Sleep 300ms
Enter
Sleep 400ms
Down
Sleep 300ms
Enter
Sleep 400ms
Type "Validate Token"
Sleep 300ms
Enter
Sleep 800ms

# Auth -> Gateway: Token Valid
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

# Gateway -> Orders: Get Orders
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
Type "Get Orders"
Sleep 300ms
Enter
Sleep 800ms

# Orders -> Gateway: Order List
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

# Gateway -> Client: Response
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

# Navigate and demonstrate selection
Type "j"
Sleep 400ms
Type "j"
Sleep 400ms
Type "j"
Sleep 400ms
Type "l
