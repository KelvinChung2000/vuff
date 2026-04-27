module m;
  assign x = cond1 ? a :
           cond2 ? b :
           c;
endmodule
// expected -----
module m;
  assign x = cond1 ? a :
             cond2 ? b :
             c;
endmodule
