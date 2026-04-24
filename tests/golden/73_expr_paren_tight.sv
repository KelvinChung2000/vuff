module m;
  logic [7:0] a, b, c, d;
  assign a = ( b + c ) * d;
  assign a = ( ( b | c ) & d );
endmodule
// expected -----
module m;
  logic [7:0] a, b, c, d;
  assign a = (b + c) * d;
  assign a = ((b | c) & d);
endmodule
