module m(
output reg a = 0,
output reg [7:0] b = 8'd42
);
endmodule
// expected -----
module m (
  output reg       a = 0,
  output reg [7:0] b = 8'd42
);
endmodule
