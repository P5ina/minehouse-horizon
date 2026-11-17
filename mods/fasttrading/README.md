# Fast Trading for Horizon

A native [Horizon](https://github.com/horizon-rust/horizon) mod (Minecraft 26.3): a port of
[Fast Trading](https://github.com/bendy1234/FastTrading) 0.2.5 (MIT). A button beside the result in a
villager's or wandering trader's window repeats the selected trade until it can't go on: out of stock, the
price went up, no payment left or no room for the result. Each tick it presses the offer again (the server
refills the payment slots) or Shift-clicks the result, then gives back what is left in the payment slots. A
result that can be damaged (tools, armour) needs Shift on the button, as the original's default block.

License: MIT.
