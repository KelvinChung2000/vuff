module m;
  assign a = {b,c,d};
endmodule
// expected -----
module m;
  assign a = {b, c, d};
endmodule
