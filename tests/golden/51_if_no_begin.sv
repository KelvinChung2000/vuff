module m;
always_comb
if (cond)
x = 1;
else
x = 0;
endmodule
// expected -----
module m;
  always_comb
    if (cond)
      x = 1;
    else
      x = 0;
endmodule
