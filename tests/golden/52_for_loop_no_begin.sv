module m;
initial
for (i = 0; i < 8; i++)
arr[i] = 0;
endmodule
// expected -----
module m;
  initial
    for (i = 0; i < 8; i++)
      arr[i] = 0;
endmodule
