module m(
input wire signed [31:0] a,
input wire [7:0] b
);
endmodule
// expected -----
module m (
  input wire signed [31:0] a,
  input wire        [ 7:0] b
);
endmodule
