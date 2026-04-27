module m;
  assign x = a ? 1 :
           long_cond ? 2 :
           3;
endmodule
// expected -----
module m;
  assign x = a         ? 1 :
             long_cond ? 2 :
             3;
endmodule
