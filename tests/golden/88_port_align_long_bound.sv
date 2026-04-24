module m(
input reg [loooong_name-1:0] a,
input logic [3:0] b
);
endmodule
// expected -----
module m (
  input reg   [loooong_name-1:0] a,
  input logic [             3:0] b
);
endmodule
