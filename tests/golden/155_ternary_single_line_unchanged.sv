module m;
  assign x = cond ? a : b;
endmodule
// expected -----
module m;
  assign x = cond ? a : b;
endmodule
